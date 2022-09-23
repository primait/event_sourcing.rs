use async_trait::async_trait;
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

use esrs::postgres::Projector;
use esrs::StoreEvent;

use crate::aggregates::{AggregateA, AggregateB};
use crate::structs::{CounterError, EventA, EventB};

#[derive(Clone)]
pub struct CounterProjector;

// This is a projector template that will project AggregateA events into a shared projection (DB table).
#[async_trait]
impl Projector<AggregateA> for CounterProjector {
    async fn project(&self, event: &StoreEvent<EventA>, connection: &mut PgConnection) -> Result<(), CounterError> {
        let existing: Option<Counter> = Counter::by_id(event.aggregate_id, &mut *connection).await?;

        match event.payload() {
            EventA::Inner => match existing {
                Some(counter) => {
                    Ok(Counter::update(event.aggregate_id, CounterUpdate::A(counter.count_a + 1), connection).await?)
                }
                None => Ok(Counter::insert(event.aggregate_id, 1, 0, connection).await?),
            },
        }
    }

    async fn delete(&self, _aggregate_id: Uuid, _connection: &mut PgConnection) -> Result<(), CounterError> {
        todo!()
    }
}

#[async_trait]
impl Projector<AggregateB> for CounterProjector {
    async fn project(&self, event: &StoreEvent<EventB>, connection: &mut PgConnection) -> Result<(), CounterError> {
        let existing: Option<Counter> = Counter::by_id(event.aggregate_id, &mut *connection).await?;

        match event.payload() {
            EventB::Inner => match existing {
                Some(counter) => {
                    Ok(Counter::update(event.aggregate_id, CounterUpdate::B(counter.count_b + 1), connection).await?)
                }
                None => Ok(Counter::insert(event.aggregate_id, 0, 1, connection).await?),
            },
        }
    }

    async fn delete(&self, _aggregate_id: Uuid, _connection: &mut PgConnection) -> Result<(), CounterError> {
        todo!()
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct Counter {
    pub counter_id: Uuid,
    pub count_a: i32,
    pub count_b: i32,
}

#[derive(Debug, Clone)]
enum CounterUpdate {
    A(i32),
    B(i32),
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

    async fn insert(
        id: Uuid,
        count_a: i32,
        count_b: i32,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("INSERT INTO counters (counter_id, count_a, count_b) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(count_a)
            .bind(count_b)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    async fn update(
        id: Uuid,
        update: CounterUpdate,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        let (val, query) = match update {
            CounterUpdate::A(val) => (
                val,
                sqlx::query_as::<_, Self>("UPDATE counters SET count_a = $2 WHERE counter_id = $1"),
            ),
            CounterUpdate::B(val) => (
                val,
                sqlx::query_as::<_, Self>("UPDATE counters SET count_b = $2 WHERE counter_id = $1"),
            ),
        };

        query.bind(id).bind(val).fetch_optional(executor).await.map(|_| ())
    }
}
