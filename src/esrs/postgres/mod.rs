use std::convert::TryInto;
use std::future::Future;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::StreamExt;
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{Executor, Pool, Postgres, Transaction};
use uuid::Uuid;

use statement::Statements;

use crate::aggregate;
use crate::store::{EventStore, StoreEvent};
use crate::types::SequenceNumber;

mod event;
pub mod policy;
pub mod projector;
mod statement;

#[cfg(test)]
mod tests;

type Projector<A> = Box<dyn projector::Projector<A> + Send + Sync>;
type Policy<A> = Box<dyn policy::Policy<A> + Send + Sync>;

pub struct PgStore<Aggregate>
where
    Aggregate: aggregate::Aggregate,
{
    pool: Pool<Postgres>,
    statements: Statements,
    projectors: Vec<Projector<Aggregate>>,
    policies: Vec<Policy<Aggregate>>,
}

impl<Aggregate> PgStore<Aggregate>
where
    Aggregate: aggregate::Aggregate,
{
    // TODO: doc
    pub fn new(pool: &Pool<Postgres>, projectors: Vec<Projector<Aggregate>>, policies: Vec<Policy<Aggregate>>) -> Self {
        Self {
            pool: pool.clone(),
            statements: Statements::new(Aggregate::name()),
            projectors,
            policies,
        }
    }

    // TODO: doc
    pub async fn setup(self) -> Result<Self, Aggregate::Error> {
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
        event: Aggregate::Event,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<StoreEvent<Aggregate::Event>, Aggregate::Error> {
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
    pub fn get_all(&self) -> BoxStream<Result<StoreEvent<Aggregate::Event>, Aggregate::Error>> {
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
    pub fn projectors(&self) -> &[Projector<Aggregate>] {
        &self.projectors
    }

    // TODO: doc
    pub fn policies(&self) -> &[Policy<Aggregate>] {
        &self.policies
    }

    // TODO: doc
    pub async fn persist_fn<'a, F: Send, T>(
        &'a self,
        fun: F,
    ) -> Result<Vec<StoreEvent<Aggregate::Event>>, Aggregate::Error>
    where
        F: FnOnce(&'a Pool<Postgres>) -> T,
        T: Future<Output = Result<Vec<StoreEvent<Aggregate::Event>>, Aggregate::Error>> + Send,
    {
        fun(&self.pool).await
    }

    // TODO: doc
    pub async fn close(&self) {
        self.pool.close().await
    }
}

#[async_trait]
impl<Aggregate> EventStore<Aggregate> for PgStore<Aggregate>
where
    Aggregate: aggregate::Aggregate,
{
    async fn by_aggregate_id(&self, aggregate_id: Uuid) -> Result<Vec<StoreEvent<Aggregate::Event>>, Aggregate::Error> {
        Ok(sqlx::query_as::<_, event::Event>(self.statements.by_aggregate_id())
            .bind(aggregate_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into()?))
            .collect::<Result<Vec<StoreEvent<Aggregate::Event>>, Aggregate::Error>>()?)
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<Aggregate::Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Aggregate::Event>>, Aggregate::Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;
        let occurred_on: DateTime<Utc> = Utc::now();
        let mut store_events: Vec<StoreEvent<Aggregate::Event>> = vec![];

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

    async fn delete_by_aggregate_id(&self, aggregate_id: Uuid) -> Result<(), Aggregate::Error> {
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
