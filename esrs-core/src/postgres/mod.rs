use std::convert::TryInto;

use async_trait::async_trait;
use chrono::Utc;
use futures::stream::BoxStream;
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Postgres, postgres::PgPoolOptions, Transaction};
use uuid::Uuid;

use crate::policy::Policy;
use crate::postgres::event::Event;
pub use crate::postgres::projector::PgProjector;
use crate::query;
use crate::SequenceNumber;
use crate::store::{EventStore, ProjectEvent, StoreEvent};

mod event;
mod projector;
mod util;

/// TODO: some doc here
pub struct PgStore<
    Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
    Err: From<sqlx::Error> + From<serde_json::Error>,
> {
    aggregate_name: String,
    pool: Pool<Postgres>,
    select: String,
    insert: String,
    projectors: Vec<Box<dyn PgProjector<Evt, Err> + Send + Sync>>,
    policies: Vec<Box<dyn Policy<Evt, Err> + Send + Sync>>,
}

impl<
    'a,
    Evt: 'a + Serialize + DeserializeOwned + Clone + Send + Sync,
    Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
> PgStore<Evt, Err>
{
    /// Prefer this. Pool should be shared between stores
    pub async fn new(
        pool: &'a Pool<Postgres>,
        // aggregate: &'a dyn Identifiable,
        name: &'a str,
        projectors: Vec<Box<dyn PgProjector<Evt, Err> + Send + Sync>>,
    ) -> Result<Self, Err> {
        // Check if table and indexes exist and eventually create them
        let _ = util::run_preconditions(pool, name).await?;

        Ok(Self {
            aggregate_name: name.to_string(),
            pool: pool.clone(),
            select: query::select_statement(name),
            insert: query::insert_statement(name),
            projectors,
            policies: vec![],
        })
    }

    pub async fn new_from_url(
        database_url: &'a str,
        name: &'a str,
        projectors: Vec<Box<dyn PgProjector<Evt, Err> + Send + Sync>>,
    ) -> Result<Self, Err> {
        let pool: Pool<Postgres> = PgPoolOptions::new().connect(database_url).await?;
        // Check if table and indexes exist and eventually create them
        let _ = util::run_preconditions(&pool, name).await?;

        Ok(Self {
            aggregate_name: name.to_string(),
            pool,
            select: query::select_statement(name),
            insert: query::insert_statement(name),
            projectors,
            policies: vec![],
        })
    }

    pub fn add_projector(&mut self, projector: Box<dyn PgProjector<Evt, Err> + Send + Sync>) -> &mut Self {
        self.projectors.push(projector);
        self
    }

    pub fn add_policy(&mut self, policy: Box<dyn Policy<Evt, Err> + Send + Sync>) -> &mut Self {
        self.policies.push(policy);
        self
    }

    /// Begin a new transaction. Commit returned transaction or Drop will automatically rollback it
    pub async fn begin<'b>(&self) -> Result<Transaction<'b, Postgres>, sqlx::Error> {
        self.pool.begin().await
    }

    pub async fn rebuild_events(&self) -> Result<(), Err> {
        let query: String = query::select_all_statement(&self.aggregate_name);

        let mut events: BoxStream<Result<Event, sqlx::Error>> =
            sqlx::query_as::<_, Event>(query.as_str()).fetch(&self.pool);

        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

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
> EventStore<Evt, Err> for PgStore<Evt, Err>
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
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        let store_event_result: Result<StoreEvent<Evt>, Err> = sqlx::query_as::<_, Event>(&self.insert)
            .bind(Uuid::new_v4())
            .bind(aggregate_id)
            .bind(serde_json::to_value(event.clone()).unwrap())
            .bind(Utc::now())
            .bind(sequence_number)
            .fetch_one(&mut transaction)
            .await
            .map_err(|error| error.into())
            .and_then(|e| e.try_into().map_err(|err: serde_json::Error| err.into()));

        let rebuild_result: Result<StoreEvent<Evt>, Err> = match store_event_result {
            Ok(store_event) => self
                .project_event(&store_event, &mut transaction)
                .await
                .map(|()| store_event),
            Err(error) => Err(error),
        };

        match rebuild_result {
            Ok(event) => {
                transaction.commit().await?;

                for policy in &self.policies {
                    policy.handle_event(&event).await?
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

#[async_trait]
impl<
    'c,
    Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
    Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
> ProjectEvent<Evt, Transaction<'c, Postgres>, Err> for PgStore<Evt, Err>
{
    async fn project_event(
        &self,
        store_event: &StoreEvent<Evt>,
        executor: &mut Transaction<'c, Postgres>,
    ) -> Result<(), Err> {
        for projector in &self.projectors {
            projector.project(store_event, executor).await?
        }
        Ok(())
    }
}
