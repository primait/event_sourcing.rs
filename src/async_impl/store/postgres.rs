use std::convert::TryInto;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::PgDone;
pub use sqlx::postgres::PgPoolOptions;
pub use sqlx::Pool;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::async_impl::policy::Policy;
use crate::async_impl::projector::Projector;
use crate::async_impl::store::StoreEvent;
use crate::{query, SequenceNumber, StoreParams};

use super::EventStore;

/// TODO: some doc here
pub struct PostgreStore<
    Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
    Err: From<sqlx::Error> + From<serde_json::Error>,
> {
    aggregate_name: String,
    pool: Pool<Postgres>,
    select: String,
    insert: String,
    projectors: Vec<Box<dyn Projector<Evt, Err> + Send + Sync>>,
    policies: Vec<Box<dyn Policy<Evt, Err> + Send + Sync>>,
}

impl<
        'a,
        Evt: 'a + Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    > PostgreStore<Evt, Err>
{
    /// Prefer this. Pool could be shared between stores
    pub async fn new(
        pool: &'a Pool<Postgres>,
        // aggregate: &'a dyn Identifiable,
        name: &'a str,
        projectors: Vec<Box<dyn Projector<Evt, Err> + Send + Sync>>,
    ) -> Result<Self, Err> {
        // Check if table and indexes exist and eventually create them
        let _ = run_preconditions(pool, name).await?;

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
        projectors: Vec<Box<dyn Projector<Evt, Err> + Send + Sync>>,
    ) -> Result<Self, Err> {
        let pool: Pool<Postgres> = PgPoolOptions::new().connect(database_url).await?;
        // Check if table and indexes exist and eventually create them
        let _ = run_preconditions(&pool, name).await?;

        Ok(Self {
            aggregate_name: name.to_string(),
            pool,
            select: query::select_statement(name),
            insert: query::insert_statement(name),
            projectors,
            policies: vec![],
        })
    }

    pub async fn new_from_params(
        params: StoreParams<'a>,
        // aggregate: &'a dyn Identifiable,
        name: &'a str,
        projectors: Vec<Box<dyn Projector<Evt, Err> + Send + Sync>>,
    ) -> Result<Self, Err> {
        Self::new_from_url(params.postgres_url().as_str(), name, projectors).await
    }

    pub fn add_projector(&mut self, projector: Box<dyn Projector<Evt, Err> + Send + Sync>) -> &mut Self {
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

    async fn persist_and_project<'b>(
        &self,
        aggregate_id: Uuid,
        event: Evt,
        sequence_number: SequenceNumber,
        transaction: &mut Transaction<'b, Postgres>,
    ) -> Result<StoreEvent<Evt>, Err> {
        let store_event: StoreEvent<Evt> = sqlx::query_as::<_, Event>(&self.insert)
            .bind(Uuid::new_v4())
            .bind(aggregate_id)
            .bind(serde_json::to_value(event).unwrap())
            .bind(Utc::now())
            .bind(sequence_number)
            .fetch_one(transaction)
            .await?
            .try_into()?;

        self.rebuild_event(&store_event).await?;

        Ok(store_event)
    }

    pub async fn rebuild_events(&self) -> Result<(), Err> {
        let query: String = query::select_all_statement(&self.aggregate_name);

        let mut events: BoxStream<Result<Event, sqlx::Error>> =
            sqlx::query_as::<_, Event>(query.as_str()).fetch(&self.pool);

        while let Some(event) = events.try_next().await? {
            let evt: StoreEvent<Evt> = event.try_into()?;
            self.rebuild_event(&evt).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl<
        Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    > EventStore<Evt, Err> for PostgreStore<Evt, Err>
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

        match self
            .persist_and_project(aggregate_id, event, sequence_number, &mut transaction)
            .await
        {
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

    async fn rebuild_event(&self, store_event: &StoreEvent<Evt>) -> Result<(), Err> {
        for projector in &self.projectors {
            projector.project(store_event).await?
        }
        Ok(())
    }

    async fn close(&self) {
        self.pool.close().await
    }
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
struct Event {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
}

impl<E: Serialize + DeserializeOwned + Clone + Send + Sync> TryInto<StoreEvent<E>> for Event {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<StoreEvent<E>, Self::Error> {
        Ok(StoreEvent {
            id: self.id,
            aggregate_id: self.aggregate_id,
            payload: serde_json::from_value::<E>(self.payload)?,
            occurred_on: self.occurred_on,
            sequence_number: self.sequence_number,
        })
    }
}

async fn run_preconditions(pool: &Pool<Postgres>, aggregate_name: &str) -> Result<(), sqlx::Error> {
    // Create table if not exists
    let _: PgDone = sqlx::query(query::create_table_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    // Create 2 indexes if not exist
    let _: PgDone = sqlx::query(query::create_id_index_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    let _: PgDone = sqlx::query(query::create_aggregate_id_index_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    Ok(())
}
