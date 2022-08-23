use async_trait::async_trait;
use sqlx::{Executor, Sqlite};
use uuid::Uuid;

use esrs::projector::{SqliteProjector};
use esrs::store::StoreEvent;

use sqlx::pool::PoolConnection;

use crate::structs::{CounterEvent, CounterError};

pub struct CounterProjector;

#[async_trait]
impl SqliteProjector<CounterEvent, CounterError> for CounterProjector {
    async fn project(
        &self,
        event: &StoreEvent<CounterEvent>,
        connection: &mut PoolConnection<Sqlite>,
    ) -> Result<(), CounterError> {
        let existing: Option<Counter> = Counter::by_id(event.aggregate_id, &mut *connection).await?.map(|existing| existing);
        match event.payload {
            CounterEvent::Incremented => {
                match existing {
                    Some(counter) => {
                        Ok(Counter::update(event.aggregate_id, counter.count + 1, &mut *connection).await?)
                    }
                    None => Ok(Counter::insert(event.aggregate_id, 1, &mut *connection).await?),
                }
            }
            CounterEvent::Decremented => {
                match existing {
                    Some(counter) => {
                        Ok(Counter::update(event.aggregate_id, counter.count - 1, &mut *connection).await?)
                    }
                    None => Ok(Counter::insert(event.aggregate_id, -1, &mut *connection).await?),
                }
            }
        }
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct Counter {
    pub counter_id: Uuid,
    pub count: i32,
}

impl Counter {
    pub async fn by_id(
        id: Uuid,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM counters WHERE counter_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    pub async fn insert(
        id: Uuid,
        count: i32,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("INSERT INTO counters (counter_id, count) VALUES ($1, $2)")
            .bind(id)
            .bind(count)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn update(
        id: Uuid,
        count: i32,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("UPDATE counters SET count = $2 WHERE counter_id = $1")
            .bind(id)
            .bind(count)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn delete(
        id: Uuid,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("DELETE FROM counter WHERE counter_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }
}
