use std::sync::Arc;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Executor, Sqlite};
use tokio::sync::Mutex;
use uuid::Uuid;

use esrs::projector::SqliteProjector;
use esrs::store::StoreEvent;

use sqlx::pool::PoolConnection;

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
impl<T: Clone + Into<ProjectorEvent> + Send + Sync + Serialize + DeserializeOwned> SqliteProjector<T, CounterError>
    for CounterProjector
{
    async fn project(
        &self,
        event: &StoreEvent<T>,
        connection: &mut PoolConnection<Sqlite>,
    ) -> Result<(), CounterError> {
        let existing: Option<Counter> = Counter::by_id(event.aggregate_id, &mut *connection)
            .await?
            .map(|existing| existing);
        let payload: ProjectorEvent = event.payload.clone().into();
        match payload {
            ProjectorEvent::A => match existing {
                Some(counter) => Ok(Counter::update(
                    event.aggregate_id,
                    counter.count_a + 1,
                    counter.count_b,
                    &mut *connection,
                )
                .await?),
                None => Ok(Counter::insert(event.aggregate_id, 1, 0, &mut *connection).await?),
            },
            ProjectorEvent::B => match existing {
                Some(counter) => Ok(Counter::update(
                    event.aggregate_id,
                    counter.count_a,
                    counter.count_b + 1,
                    &mut *connection,
                )
                .await?),
                None => Ok(Counter::insert(event.aggregate_id, 0, 1, &mut *connection).await?),
            },
        }
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct Counter {
    pub counter_id: Uuid,
    pub count_a: i32,
    pub count_b: i32,
}

impl Counter {
    pub async fn by_id(id: Uuid, executor: impl Executor<'_, Database = Sqlite>) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM counters WHERE counter_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    pub async fn insert(
        id: Uuid,
        count_a: i32,
        count_b: i32,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("INSERT INTO counters (counter_id, count_a, count_b) VALUES ($1, $2, $3)")
            .bind(id)
            .bind(count_a)
            .bind(count_b)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn update(
        id: Uuid,
        count_a: i32,
        count_b: i32,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("UPDATE counters SET count_a = $2, count_b = $3 WHERE counter_id = $1")
            .bind(id)
            .bind(count_a)
            .bind(count_b)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn delete(id: Uuid, executor: impl Executor<'_, Database = Sqlite>) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("DELETE FROM counter WHERE counter_id = $1")
            .bind(id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }
}

// Here's an example of how one might implement a projector that is shared between two aggregates, in a
// way that guarantees soundness when both aggregate event types can lead to modifying a shared
// column in the projection. Note that this requires a different Aggregate setup, where a shared
// (inner) projector is constructed, two SharedProjectors are constructed to wrap the inner, and then
// an EventStore for each event type is constructed and passed the right SharedProjector for it's Event type,
// before being passed into some Aggregate::new. It also, requires cloning every event
pub struct SharedProjector<InnerEvent, InnerError> {
    inner: Arc<Mutex<dyn SqliteProjector<InnerEvent, InnerError> + Send + Sync>>,
}

impl<InnerEvent, InnerError> SharedProjector<InnerEvent, InnerError> {
    pub fn new(inner: Arc<Mutex<dyn SqliteProjector<InnerEvent, InnerError> + Send + Sync>>) -> Self {
        SharedProjector { inner: inner }
    }
}

#[async_trait]
impl<Event, Error, InnerEvent, InnerError> SqliteProjector<Event, Error> for SharedProjector<InnerEvent, InnerError>
where
    Event: Into<InnerEvent> + Clone + Send + Sync + Serialize + DeserializeOwned,
    InnerEvent: Send + Sync + Serialize + DeserializeOwned,
    InnerError: Into<Error>,
{
    async fn project(&self, event: &StoreEvent<Event>, connection: &mut PoolConnection<Sqlite>) -> Result<(), Error> {
        let event: StoreEvent<InnerEvent> = event.clone().map(Event::into);
        let inner = self.inner.lock().await;
        let result = inner.project(&event, connection).await;
        match result {
            Ok(()) => Ok(()),
            Err(e) => Err(e.into()),
        }
    }
}
