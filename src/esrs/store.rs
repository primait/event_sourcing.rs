use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::types::SequenceNumber;
use crate::{Aggregate, AggregateManager};

/// An EventStore is responsible for persisting events that an aggregate emits into a database, and loading the events
/// that represent an aggregate's history from the database.
#[async_trait]
pub trait EventStore {
    type Manager: AggregateManager;

    /// Loads the events that an aggregate instance has emitted in the past.
    async fn by_aggregate_id(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<StoreEvent<<Self::Manager as Aggregate>::Event>>, <Self::Manager as Aggregate>::Error>;

    /// Persists multiple events into the database. This should be done in a single transaction - either
    /// all the events are persisted correctly, or none are.
    ///
    /// Persisting events may additionally trigger configured Projectors.
    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<<Self::Manager as Aggregate>::Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<<Self::Manager as Aggregate>::Event>>, <Self::Manager as Aggregate>::Error>;

    /// Delete all events from events store related to given `aggregate_id`.
    ///
    /// Moreover it should delete all the projections.
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), <Self::Manager as Aggregate>::Error>;
}

/// A `StoreEvent` contains the payload (the original event) alongside the event's metadata.
#[derive(Debug)]
pub struct StoreEvent<Event> {
    /// Uniquely identifies an event among all events emitted from all aggregates.
    pub id: Uuid,
    /// The aggregate instance that emitted the event.
    pub aggregate_id: Uuid,
    /// The original, emitted, event.
    pub payload: Event,
    /// The timestamp of when the event is persisted.
    pub occurred_on: DateTime<Utc>,
    /// The sequence number of the event, within its specific aggregate instance.
    pub sequence_number: SequenceNumber,
}

impl<Event> StoreEvent<Event> {
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub const fn payload(&self) -> &Event {
        &self.payload
    }
}

/// Default generic implementation for every type [`Box<dyn EventStore>`]. This is particularly useful
/// when there's the need in your codebase to have a generic [`EventStore`] inside your [`Aggregate`].
///
/// # Example
///
/// ```ignore
/// pub struct MyAggregate {
///     event_store: Box<dyn esrs::EventStore<Manager = Self>>,
/// }
///
/// // Your [`Aggregate`] impl here
///
/// impl esrs::AggregateManager for MyAggregate {
///     type EventStore = Box<dyn esrs::EventStore<Manager = Self>>;
///
///     fn name() -> &'static str where Self: Sized {
///         "whatever"
///     }
///
///     fn event_store(&self) -> &Self::EventStore {
///         self.event_store.as_ref()
///     }
/// }
/// ```
#[async_trait]
impl<M> EventStore for Box<dyn EventStore<Manager = M> + Send + Sync>
where
    M: AggregateManager,
{
    type Manager = M;

    async fn by_aggregate_id(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<StoreEvent<<Self::Manager as Aggregate>::Event>>, <Self::Manager as Aggregate>::Error> {
        self.as_ref().by_aggregate_id(aggregate_id).await
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<<Self::Manager as Aggregate>::Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<<Self::Manager as Aggregate>::Event>>, <Self::Manager as Aggregate>::Error> {
        self.as_ref()
            .persist(aggregate_id, events, starting_sequence_number)
            .await
    }

    async fn delete(&self, aggregate_id: Uuid) -> Result<(), <Self::Manager as Aggregate>::Error> {
        self.as_ref().delete(aggregate_id).await
    }
}
