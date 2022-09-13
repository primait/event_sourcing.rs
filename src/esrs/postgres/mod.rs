use std::convert::TryInto;
use std::future::Future;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{Executor, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::aggregate::Aggregate;
use crate::esrs::store::{EventStore, StoreEvent};
use crate::esrs::{event, SequenceNumber};

pub mod policy;
pub mod projector;

#[cfg(test)]
mod tests;

type Projector<Event, Error> = Box<dyn projector::Projector<Event, Error> + Send + Sync>;
type Policy<Event, Error> = Box<dyn policy::Policy<Event, Error> + Send + Sync>;

pub struct PgStore<Event, Error> {
    table_name: String,
    pool: Pool<Postgres>,
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
            table_name: format!("{}_events", A::name()),
            pool: pool.clone(),
            projectors,
            policies,
        }
    }

    // TODO: doc
    pub async fn setup(self) -> Result<Self, Error> {
        create_table::<Error>(&self.table_name, &self.pool).await?;
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

        let insert_query: String = format!(
            "INSERT INTO {} (id, aggregate_id, payload, occurred_on, sequence_number) VALUES ($1, $2, $3, $4, $5)",
            &self.table_name
        );

        let _ = sqlx::query(insert_query.as_str())
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
    pub fn projectors(&self) -> &Vec<Projector<Event, Error>> {
        &self.projectors
    }

    // TODO: doc
    pub fn policies(&self) -> &Vec<Policy<Event, Error>> {
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
        let by_aggregate_id_query: String = format!(
            "SELECT * FROM {} WHERE aggregate_id = $1 ORDER BY sequence_number ASC",
            &self.table_name
        );

        Ok(sqlx::query_as::<_, event::Event>(by_aggregate_id_query.as_str())
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

        let delete_by_aggregate_id_query: String = format!("DELETE FROM {} WHERE aggregate_id = $1", &self.table_name);

        let _ = sqlx::query(delete_by_aggregate_id_query.as_str())
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

    fn get_all(&self) -> BoxStream<Result<StoreEvent<Event>, Error>> {
        Box::pin(
            sqlx::query_as::<_, event::Event>(self.queries.select_all())
                .fetch(&self.pool)
                .map(|res| match res {
                    Ok(e) => match e.try_into() {
                        Ok(e) => Ok(e),
                        Err(e) => Err(Error::from(e)),
                    },
                    Err(e) => Err(e.into()),
                }),
        )
    }
}

async fn create_table<Error>(table_name: &str, pool: &Pool<Postgres>) -> Result<(), Error>
where
    Error: From<sqlx::Error> + Send + Sync,
{
    // Create events table if not exists
    let create_table_query: String = format!(
        "CREATE TABLE IF NOT EXISTS {0}
            (
              id uuid NOT NULL,
              aggregate_id uuid NOT NULL,
              payload jsonb NOT NULL,
              occurred_on TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
              sequence_number INT NOT NULL DEFAULT 1,
              CONSTRAINT {0}_pkey PRIMARY KEY (id)
            )",
        table_name
    );

    let _: PgQueryResult = sqlx::query(create_table_query.as_str())
        .bind(table_name)
        .execute(pool)
        .await?;

    // Create index on aggregate_id for `by_aggregate_id` query.
    let create_aggregate_id_index_query: String = format!(
        "CREATE INDEX IF NOT EXISTS {0}_aggregate_id ON {0} USING btree (((payload ->> 'id'::text)))",
        table_name
    );

    let _: PgQueryResult = sqlx::query(create_aggregate_id_index_query.as_str())
        .bind(table_name)
        .execute(pool)
        .await?;

    // Create unique constraint `aggregate_id`-`sequence_number` to avoid race conditions.
    let create_aggregate_id_sequence_number_unique_constraint_query: String = format!(
        "CREATE UNIQUE INDEX IF NOT EXISTS {0}_aggregate_id_sequence_number ON {0}(aggregate_id, sequence_number)",
        table_name
    );

    let _: PgQueryResult = sqlx::query(create_aggregate_id_sequence_number_unique_constraint_query.as_str())
        .bind(table_name)
        .execute(pool)
        .await?;

    Ok(())
}
