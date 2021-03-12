use std::convert::TryInto;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::{PgDone, PgPoolOptions};
pub use sqlx::Pool;
pub use sqlx::Postgres;
pub use sqlx::Transaction;
use uuid::Uuid;

use crate::projector::Projector;
use crate::store::StoreEvent;
use crate::{query, SequenceNumber, StoreParams};

use super::EventStore;
use crate::policy::Policy;

/// TODO: some doc here
pub struct PostgreStore<
    Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
    Err: From<sqlx::Error> + From<serde_json::Error>,
> {
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
            pool: pool.clone(),
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
        let pool: Pool<Postgres> = PgPoolOptions::new().connect(params.postgres_url().as_str()).await?;
        // Check if table and indexes exist and eventually create them
        let _ = run_preconditions(&pool, name).await?;

        Ok(Self {
            pool,
            select: query::select_statement(name),
            insert: query::insert_statement(name),
            projectors,
            policies: vec![],
        })
    }

    pub fn add_projector(&mut self, projector: Box<dyn Projector<Evt, Err> + Send + Sync>) -> &mut Self {
        self.projectors.push(projector);
        self
    }

    pub fn add_policy(&mut self, policy: Box<dyn Policy<Evt, Err> + Send + Sync>) -> &mut Self {
        self.policies.push(policy);
        self
    }

    async fn persist_and_project(
        &self,
        aggregate_id: Uuid,
        event: Evt,
        sequence_number: SequenceNumber,
    ) -> Result<StoreEvent<Evt>, Err> {
        let store_event: StoreEvent<Evt> = sqlx::query_as::<_, Event>(&self.insert)
            .bind(Uuid::new_v4())
            .bind(aggregate_id)
            .bind(serde_json::to_value(event).unwrap())
            .bind(Utc::now())
            .bind(sequence_number)
            .fetch_one(&self.pool)
            .await?
            .try_into()?;

        self.rebuild_event(&store_event).await?;

        Ok(store_event)
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
        let transaction: Transaction<Postgres> = self.pool.begin().await?;

        match self.persist_and_project(aggregate_id, event, sequence_number).await {
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
