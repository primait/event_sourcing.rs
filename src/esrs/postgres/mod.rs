use std::convert::TryInto;
use std::future::Future;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::StreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{Executor, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::aggregate::Aggregate;
use crate::esrs::postgres::statement::Statements;
use crate::esrs::store::{EventStore, StoreEvent};
use crate::esrs::{event, SequenceNumber};

pub mod policy;
pub mod projector;

mod statement;
#[cfg(test)]
mod tests;

type Projector<Event, Error> = Box<dyn projector::Projector<Event, Error> + Send + Sync>;
type Policy<Event, Error> = Box<dyn policy::Policy<Event, Error> + Send + Sync>;

pub struct PgStore<Event, Error> {
    pool: Pool<Postgres>,
    statements: Statements,
    projectors: Vec<Projector<Event, Error>>,
    policies: Vec<Policy<Event, Error>>,
}

impl<Event, Error> PgStore<Event, Error>
where
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
{
    // TODO: doc
    pub fn new<A: Aggregate>(
        pool: &Pool<Postgres>,
        projectors: Vec<Projector<Event, Error>>,
        policies: Vec<Policy<Event, Error>>,
    ) -> Self {
        Self {
            pool: pool.clone(),
            statements: Statements::new(A::name()),
            projectors,
            policies,
        }
    }

    // TODO: doc
    pub async fn setup(self) -> Result<Self, Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        // Create events table if not exists
        let _: PgQueryResult = sqlx::query(self.statements.create_table())
            .execute(&mut *transaction)
            .await?;

        // Create index on aggregate_id for `by_aggregate_id` query.
        let _: PgQueryResult = sqlx::query(self.statements.create_index())
            .execute(&mut *transaction)
            .await?;

        // Create unique constraint `aggregate_id`-`sequence_number` to avoid race conditions.
        let _: PgQueryResult = sqlx::query(self.statements.create_unique_constraint())
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(self)
    }

    // TODO: doc
    pub async fn insert(
        &self,
        aggregate_id: Uuid,
        event: Event,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<StoreEvent<Event>, Error> {
        let id: Uuid = Uuid::new_v4();

        let _ = sqlx::query(self.statements.insert())
            .bind(id)
            .bind(aggregate_id)
            .bind(Json(&event))
            .bind(occurred_on)
            .bind(sequence_number)
            .execute(executor)
            .await?;

        Ok(StoreEvent {
            id,
            aggregate_id,
            payload: event,
            occurred_on,
            sequence_number,
        })
    }

    // TODO: doc
    pub fn get_all(&self) -> BoxStream<Result<StoreEvent<Event>, Error>> {
        Box::pin({
            sqlx::query_as::<_, event::Event>(self.statements.select_all())
                .fetch(&self.pool)
                .map(|res| match res {
                    Ok(event) => event.try_into().map_err(Into::into),
                    Err(error) => Err(error.into()),
                })
        })
    }

    // TODO: doc
    pub fn projectors(&self) -> &[Projector<Event, Error>] {
        &self.projectors
    }

    // TODO: doc
    pub fn policies(&self) -> &[Policy<Event, Error>] {
        &self.policies
    }

    // TODO: doc
    pub async fn persist_fn<'a, F: Send, T>(&'a self, fun: F) -> Result<Vec<StoreEvent<Event>>, Error>
    where
        F: FnOnce(&'a Pool<Postgres>) -> T,
        T: Future<Output = Result<Vec<StoreEvent<Event>>, Error>> + Send,
    {
        fun(&self.pool).await
    }

    // TODO: doc
    pub async fn close(&self) {
        self.pool.close().await
    }
}

#[async_trait]
impl<Event, Error> EventStore<Event, Error> for PgStore<Event, Error>
where
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
{
    async fn by_aggregate_id(&self, aggregate_id: Uuid) -> Result<Vec<StoreEvent<Event>>, Error> {
        Ok(sqlx::query_as::<_, event::Event>(self.statements.by_aggregate_id())
            .bind(aggregate_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into()?))
            .collect::<Result<Vec<StoreEvent<Event>>, Error>>()?)
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Event>>, Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;
        let occurred_on: DateTime<Utc> = Utc::now();
        let mut store_events: Vec<StoreEvent<Event>> = vec![];

        for (index, event) in events.into_iter().enumerate() {
            store_events.push(
                self.insert(
                    aggregate_id,
                    event,
                    occurred_on,
                    starting_sequence_number + index as i32,
                    &mut *transaction,
                )
                .await?,
            )
        }

        for store_event in store_events.iter() {
            for projector in self.projectors() {
                projector.project(store_event, &mut transaction).await?;
            }
        }

        transaction.commit().await?;

        for store_event in store_events.iter() {
            for policy in self.policies() {
                policy.handle_event(store_event, &self.pool).await?;
            }
        }

        Ok(store_events)
    }

    async fn delete_by_aggregate_id(&self, aggregate_id: Uuid) -> Result<(), Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        let _ = sqlx::query(self.statements.delete_by_aggregate_id())
            .bind(aggregate_id)
            .execute(&mut *transaction)
            .await
            .map(|_| ())?;

        for projector in &self.projectors {
            projector.delete(aggregate_id, &mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}
