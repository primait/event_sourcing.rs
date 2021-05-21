use std::convert::TryInto;
use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::sqlite::{SqliteDone, SqlitePoolOptions};
use sqlx::{Pool, Sqlite, Transaction};
use uuid::Uuid;

use policy::SqlitePolicy;
use projector::SqliteProjector;

use crate::esrs::event::Event;
use crate::esrs::store::{EventStore, ProjectEvent, StoreEvent};
use crate::esrs::{query, SequenceNumber};

pub mod policy;
pub mod projector;
mod util;

/// TODO: some doc here
pub struct SqliteStore<
    Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
    Err: From<sqlx::Error> + From<serde_json::Error>,
> {
    aggregate_name: String,
    pool: Pool<Sqlite>,
    select: String,
    insert: String,
    projectors: Vec<Box<dyn SqliteProjector<Evt, Err> + Send + Sync>>,
    policies: Vec<Box<dyn SqlitePolicy<Evt, Err> + Send + Sync>>,
}

impl<
        'a,
        Evt: 'a + Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    > SqliteStore<Evt, Err>
{
    /// Prefer this. Pool should be shared between stores
    pub async fn new(
        pool: &'a Pool<Sqlite>,
        name: &'a str,
        projectors: Vec<Box<dyn SqliteProjector<Evt, Err> + Send + Sync>>,
        policies: Vec<Box<dyn SqlitePolicy<Evt, Err> + Send + Sync>>,
    ) -> Result<Self, Err> {
        // Check if table and indexes exist and eventually create them
        let _ = util::run_preconditions(pool, name).await?;

        Ok(Self {
            aggregate_name: name.to_string(),
            pool: pool.clone(),
            select: query::select_statement(name),
            insert: query::insert_statement(name),
            projectors,
            policies,
        })
    }

    pub async fn new_from_url(
        database_url: &'a str,
        name: &'a str,
        projectors: Vec<Box<dyn SqliteProjector<Evt, Err> + Send + Sync>>,
        policies: Vec<Box<dyn SqlitePolicy<Evt, Err> + Send + Sync>>,
    ) -> Result<Self, Err> {
        let pool: Pool<Sqlite> = SqlitePoolOptions::new().connect(database_url).await?;
        Self::new(&pool, name, projectors, policies).await
    }

    pub fn add_projector(&mut self, projector: Box<dyn SqliteProjector<Evt, Err> + Send + Sync>) -> &mut Self {
        self.projectors.push(projector);
        self
    }

    pub fn add_policy(&mut self, policy: Box<dyn SqlitePolicy<Evt, Err> + Send + Sync>) -> &mut Self {
        self.policies.push(policy);
        self
    }

    /// Begin a new transaction. Commit returned transaction or Drop will automatically rollback it
    pub async fn begin<'b>(&self) -> Result<Transaction<'b, Sqlite>, sqlx::Error> {
        self.pool.begin().await
    }

    pub async fn rebuild_events(&self) -> Result<(), Err> {
        let query: String = query::select_all_statement(&self.aggregate_name);

        let mut events: BoxStream<Result<Event, sqlx::Error>> =
            sqlx::query_as::<_, Event>(query.as_str()).fetch(&self.pool);

        let mut transaction: Transaction<Sqlite> = self.pool.begin().await?;

        while let Some(event) = events.try_next().await? {
            let evt: StoreEvent<Evt> = event.try_into()?;
            self.project_event(&evt, &mut transaction).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl<
        Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    > EventStore<Evt, Err> for SqliteStore<Evt, Err>
{
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Evt>>, Err> {
        Ok(sqlx::query_as::<_, Event>(&self.select)
            .bind(id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into()?))
            .collect::<Result<Vec<StoreEvent<Evt>>, Err>>()?)
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        event: Evt,
        sequence_number: SequenceNumber,
    ) -> Result<StoreEvent<Evt>, Err> {
        let mut transaction: Transaction<Sqlite> = self.pool.begin().await?;

        let event_id: Uuid = Uuid::new_v4();
        let occurred_on: DateTime<Utc> = Utc::now();
        let store_event_result: Result<SqliteDone, Err> = sqlx::query(&self.insert)
            .bind(event_id)
            .bind(aggregate_id)
            .bind(serde_json::to_value(event.clone()).unwrap())
            .bind(Utc::now())
            .bind(sequence_number)
            .execute(&mut transaction)
            .await
            .map_err(|error| error.into());

        let rebuild_result: Result<StoreEvent<Evt>, Err> = match store_event_result {
            Ok(_) => {
                let store_event: StoreEvent<Evt> = StoreEvent {
                    id: event_id,
                    aggregate_id,
                    payload: event.clone(),
                    occurred_on,
                    sequence_number,
                };

                self.project_event(&store_event, &mut transaction)
                    .await
                    .map(|()| store_event)
            }
            Err(error) => Err(error),
        };

        match rebuild_result {
            Ok(event) => {
                transaction.commit().await?;

                for policy in &self.policies {
                    policy.handle_event(&event, &self.pool).await?
                }

                Ok(event)
            }
            Err(err) => {
                transaction.rollback().await?;
                Err(err)
            }
        }
    }

    async fn close(&self) {
        self.pool.close().await
    }
}

impl<
        'c,
        Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    > ProjectEvent<Evt, Transaction<'c, Sqlite>, Err> for SqliteStore<Evt, Err>
{
    fn project_event<'a>(
        &'a self,
        store_event: &'a StoreEvent<Evt>,
        executor: &'a mut Transaction<'c, Sqlite>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
    {
        async fn commit<
            Ev: Serialize + DeserializeOwned + Clone + Send + Sync,
            Er: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        >(
            _self: &SqliteStore<Ev, Er>,
            store_event: &StoreEvent<Ev>,
            executor: &mut Transaction<'_, Sqlite>,
        ) -> Result<(), Er> {
            for projector in &_self.projectors {
                projector.project(store_event, executor).await?
            }

            Ok(())
        }

        Box::pin(commit::<Evt, Err>(self, store_event, executor))
    }
}
