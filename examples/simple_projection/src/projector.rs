use async_trait::async_trait;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use esrs::store::StoreEvent;

use esrs::projector::Projector;

use crate::structs::{CounterError, CounterEvent};

pub struct CounterProjector;

#[async_trait]
impl Projector<CounterEvent, CounterError> for CounterProjector {
    async fn project(&self, event: &StoreEvent<CounterEvent>) -> Result<(), CounterError> {
        let existing: Option<Counter> = Counter::by_id(event.aggregate_id, todo!())
            .await?
            .map(|existing| existing);
        match event.payload {
            CounterEvent::Incremented => match existing {
                Some(counter) => Ok(Counter::update(event.aggregate_id, counter.count + 1, todo!()).await?),
                None => Ok(Counter::insert(event.aggregate_id, 1, todo!()).await?),
            },
            CounterEvent::Decremented => match existing {
                Some(counter) => Ok(Counter::update(event.aggregate_id, counter.count - 1, todo!()).await?),
                None => Ok(Counter::insert(event.aggregate_id, -1, todo!()).await?),
            },
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
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM counters WHERE counter_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    pub async fn insert(
        id: Uuid,
        count: i32,
        executor: impl Executor<'_, Database = Postgres>,
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
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("UPDATE counters SET count = $2 WHERE counter_id = $1")
            .bind(id)
            .bind(count)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn delete(id: Uuid, executor: impl Executor<'_, Database = Postgres>) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("DELETE FROM counter WHERE counter_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }
}
