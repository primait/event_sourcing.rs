use std::convert::TryInto;
use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{Executor, Postgres, Sqlite};
use uuid::Uuid;

use crate::esrs::store::StoreEvent;
use crate::esrs::SequenceNumber;

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug)]
pub struct Event {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
}

impl<E: Serialize + DeserializeOwned + Send + Sync> TryInto<StoreEvent<E>> for Event {
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

#[cfg(any(feature = "postgres", feature = "sqlite"))]
pub trait Querier<Database: sqlx::Database> {
    fn insert<'a, Evt, Err>(
        aggregate_name: &'a str,
        event_id: Uuid,
        aggregate_id: Uuid,
        event: &'a Evt,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'a, Database = Database> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync;

    fn by_aggregate_id<'a, Evt, Err>(
        aggregate_name: &'a str,
        aggregate_id: Uuid,
        executor: impl Executor<'a, Database = Database> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StoreEvent<Evt>>, Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync;

    fn delete_by_aggregate_id<'a, Evt, Err>(
        aggregate_name: &'a str,
        aggregate_id: Uuid,
        executor: impl Executor<'a, Database = Database> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync;
}

#[cfg(feature = "postgres")]
impl Querier<Postgres> for Event {
    fn insert<'a, Evt, Err>(
        aggregate_name: &'a str,
        event_id: Uuid,
        aggregate_id: Uuid,
        event: &'a Evt,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'a, Database = Postgres> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    {
        Box::pin(async move {
            Ok(sqlx::query(insert_query(aggregate_name).as_str())
                .bind(event_id)
                .bind(aggregate_id)
                .bind(Json(event))
                .bind(occurred_on)
                .bind(sequence_number)
                .execute(executor)
                .await
                .map(|_| ())?)
        })
    }

    fn by_aggregate_id<'a, Evt, Err>(
        aggregate_name: &'a str,
        aggregate_id: Uuid,
        executor: impl Executor<'a, Database = Postgres> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StoreEvent<Evt>>, Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    {
        Box::pin(async move {
            Ok(
                sqlx::query_as::<_, Event>(by_aggregate_id_query(aggregate_name).as_str())
                    .bind(aggregate_id)
                    .fetch_all(executor)
                    .await?
                    .into_iter()
                    .map(|event| Ok(event.try_into()?))
                    .collect::<Result<Vec<StoreEvent<Evt>>, Err>>()?,
            )
        })
    }

    fn delete_by_aggregate_id<'a, Evt, Err>(
        aggregate_name: &'a str,
        aggregate_id: Uuid,
        executor: impl Executor<'a, Database = Postgres> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    {
        Box::pin(async move {
            Ok(sqlx::query(delete_query(aggregate_name).as_str())
                .bind(aggregate_id)
                .execute(executor)
                .await
                .map(|_| ())?)
        })
    }
}

#[cfg(feature = "sqlite")]
impl Querier<Sqlite> for Event {
    fn insert<'a, Evt, Err>(
        aggregate_name: &'a str,
        event_id: Uuid,
        aggregate_id: Uuid,
        event: &'a Evt,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'a, Database = Sqlite> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    {
        Box::pin(async move {
            Ok(sqlx::query(insert_query(aggregate_name).as_str())
                .bind(event_id)
                .bind(aggregate_id)
                .bind(Json(event))
                .bind(occurred_on)
                .bind(sequence_number)
                .execute(executor)
                .await
                .map(|_| ())?)
        })
    }

    fn by_aggregate_id<'a, Evt, Err>(
        aggregate_name: &'a str,
        aggregate_id: Uuid,
        executor: impl Executor<'a, Database = Sqlite> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StoreEvent<Evt>>, Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    {
        Box::pin(async move {
            Ok(
                sqlx::query_as::<_, Event>(by_aggregate_id_query(aggregate_name).as_str())
                    .bind(aggregate_id)
                    .fetch_all(executor)
                    .await?
                    .into_iter()
                    .map(|event| Ok(event.try_into()?))
                    .collect::<Result<Vec<StoreEvent<Evt>>, Err>>()?,
            )
        })
    }

    fn delete_by_aggregate_id<'a, Evt, Err>(
        aggregate_name: &'a str,
        aggregate_id: Uuid,
        executor: impl Executor<'a, Database = Sqlite> + 'a,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    {
        Box::pin(async move {
            Ok(sqlx::query(delete_query(aggregate_name).as_str())
                .bind(aggregate_id)
                .execute(executor)
                .await
                .map(|_| ())?)
        })
    }
}

fn by_aggregate_id_query(table_name: &str) -> String {
    format!(
        "SELECT * FROM {}_events WHERE aggregate_id = $1 ORDER BY sequence_number ASC",
        table_name
    )
}

fn insert_query(aggregate_name: &str) -> String {
    format!(
        "
    INSERT INTO {}_events
    (id, aggregate_id, payload, occurred_on, sequence_number)
    VALUES ($1, $2, $3, $4, $5)
    ",
        aggregate_name
    )
}

fn delete_query(aggregate_name: &str) -> String {
    format!("DELETE FROM {}_events WHERE aggregate_id = $1", aggregate_name)
}

pub(crate) fn select_all_query(aggregate_name: &str) -> String {
    format!("SELECT * FROM {}_events ORDER BY sequence_number ASC", aggregate_name)
}
