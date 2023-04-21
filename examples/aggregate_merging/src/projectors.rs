use async_trait::async_trait;
use sqlx::{Executor, PgConnection, Postgres};
use uuid::Uuid;

use esrs::{StoreEvent, TransactionalQuery};

use crate::aggregates::{AggregateA, AggregateB};
use crate::structs::{CounterError, EventA, EventB};

#[derive(Clone)]
pub struct CounterTransactionalQuery;

// This is a projector template that will project AggregateA events into a shared projection (DB table).
#[async_trait]
impl TransactionalQuery<AggregateA, PgConnection> for CounterTransactionalQuery {
    async fn handle(&self, event: &StoreEvent<EventA>, connection: &mut PgConnection) -> Result<(), CounterError> {
        match event.payload() {
            EventA::Inner { shared_id: id } => {
                let existing = Counter::by_id(*id, &mut *connection).await?;
                match existing {
                    Some(counter) => Ok(Counter::update(*id, CounterUpdate::A(counter.count_a + 1), connection).await?),
                    None => Ok(Counter::insert(*id, Some(event.aggregate_id), None, 1, 0, connection).await?),
                }
            }
        }
    }

    async fn delete(&self, aggregate_id: Uuid, connection: &mut PgConnection) -> Result<(), CounterError> {
        Ok(Counter::delete_by_counter_a_id(aggregate_id, connection).await?)
    }
}

// This is a projector template that will project AggregateB events into a shared projection (DB table).
#[async_trait]
impl TransactionalQuery<AggregateB, PgConnection> for CounterTransactionalQuery {
    async fn handle(&self, event: &StoreEvent<EventB>, connection: &mut PgConnection) -> Result<(), CounterError> {
        match event.payload() {
            EventB::Inner { shared_id: id } => {
                let existing = Counter::by_id(*id, &mut *connection).await?;
                match existing {
                    Some(counter) => Ok(Counter::update(*id, CounterUpdate::B(counter.count_b + 1), connection).await?),
                    None => Ok(Counter::insert(*id, None, Some(event.aggregate_id), 0, 1, connection).await?),
                }
            }
        }
    }

    async fn delete(&self, aggregate_id: Uuid, connection: &mut PgConnection) -> Result<(), CounterError> {
        Ok(Counter::delete_by_counter_b_id(aggregate_id, connection).await?)
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct Counter {
    pub counter_id: Uuid,
    pub count_a: i32,
    pub count_b: i32,
}

#[derive(Clone, Debug)]
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
        count_a_id: Option<Uuid>,
        count_b_id: Option<Uuid>,
        count_a: i32,
        count_b: i32,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO counters (counter_id, counter_a_id, counter_b_id, count_a, count_b) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id)
        .bind(count_a_id)
        .bind(count_b_id)
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

    async fn delete_by_counter_a_id(
        id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("DELETE FROM counters WHERE counter_a_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    async fn delete_by_counter_b_id(
        id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("DELETE FROM counters WHERE counter_b_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }
}
