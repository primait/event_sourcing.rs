use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Executor, Postgres, Transaction};
use uuid::Uuid;

use esrs::projector::Projector;
use esrs::store::StoreEvent;

use crate::structs::{CounterError, ProjectorEvent};

pub struct CounterProjector;

// This is a projector template that will project any events that implement Into<ProjectorEvent>
// into a shared projection (DB table). This behaviour - consuming events from more than one aggregate,
// and projecting them to a shared table, is RACE PRONE - if the projector writes to the same column
// when consuming more than one kind of event (as it does in this example, writing to both count_a and
// count_b when updating the counts, regardless of if it received and EventA or an EventB), then it is
// possible for two simultaneous transactions, updating the same projection, to occur. If the projector
// also relies on previous state to calculate next state (as this one does), this is a data race. The fix
// (not implemented here, for demonstration purposes) is dependant on your database transaction isolation
// model - in this case, using queries which only update count_a when EventA is received, and count_b
// when EventB is received, would be sufficient to guarantee soundness.
#[async_trait]
impl<T: Clone + Into<ProjectorEvent> + Send + Sync + Serialize + DeserializeOwned> Projector<Postgres, T, CounterError>
    for CounterProjector
{
    async fn project(
        &self,
        event: &StoreEvent<T>,
        transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<(), CounterError> {
        let existing: Option<Counter> = Counter::by_id(event.aggregate_id, &mut *transaction).await?;
        let payload: ProjectorEvent = event.payload.clone().into();

        match payload {
            ProjectorEvent::A => match existing {
                Some(counter) => Ok(Counter::update(
                    event.aggregate_id,
                    CounterUpdate::A(counter.count_a + 1),
                    &mut *transaction,
                )
                .await?),
                None => Ok(Counter::insert(event.aggregate_id, 1, 0, &mut *transaction).await?),
            },
            ProjectorEvent::B => match existing {
                Some(counter) => Ok(Counter::update(
                    event.aggregate_id,
                    CounterUpdate::B(counter.count_b + 1),
                    &mut *transaction,
                )
                .await?),
                None => Ok(Counter::insert(event.aggregate_id, 0, 1, &mut *transaction).await?),
            },
        }
    }

    async fn delete(
        &self,
        _aggregate_id: Uuid,
        _transaction: &mut Transaction<'_, Postgres>,
    ) -> Result<(), CounterError> {
        Ok(())
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

    pub async fn delete(id: Uuid, executor: impl Executor<'_, Database = Postgres>) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("DELETE FROM counters WHERE counter_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }
}
