use std::ops::Deref;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::state::AggregateState;
use crate::types::SequenceNumber;

#[cfg(feature = "postgres")]
pub mod postgres;

/// Marker trait for every [`EventStoreLockGuard`].
///
/// Implementors should unlock concurrent access to the guarded resource, when dropped.
pub trait UnlockOnDrop: Send + Sync + 'static {}

/// Lock guard preventing concurrent access to a resource.
///
/// The lock is released when this guard is dropped.
pub struct EventStoreLockGuard(Box<dyn UnlockOnDrop>);

impl EventStoreLockGuard {
    /// Creates a new instance from any [`UnlockOnDrop`].
    #[must_use]
    pub fn new(lock: impl UnlockOnDrop) -> Self {
        Self(Box::new(lock))
    }
}

/// An EventStore is responsible for persisting events that an aggregate emits into a database, and loading the events
/// that represent an aggregate's history from the database.
#[async_trait]
pub trait EventStore {
    type Aggregate: crate::Aggregate;
    type Error: std::error::Error;

    /// Acquires a lock for the given aggregate, or waits for outstanding guards to be released.
    ///
    /// Used to prevent concurrent access to the aggregate state.
    /// Note that any process which does *not* `lock` will get immediate (possibly shared!) access.
    /// ALL accesses (regardless of this guard) are subject to the usual optimistic locking strategy on write.
    async fn lock(&self, aggregate_id: Uuid) -> Result<EventStoreLockGuard, Self::Error>;

    /// Loads the events that an aggregate instance has emitted in the past.
    async fn by_aggregate_id(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<StoreEvent<<Self::Aggregate as crate::Aggregate>::Event>>, Self::Error>;

    /// Persists multiple events into the database. This should be done in a single transaction - either
    /// all the events are persisted correctly, or none are.
    ///
    /// Persisting events may additionally trigger configured event handlers (transactional and non-transactional).
    async fn persist(
        &self,
        aggregate_state: &mut AggregateState<<Self::Aggregate as crate::Aggregate>::State>,
        events: Vec<<Self::Aggregate as crate::Aggregate>::Event>,
    ) -> Result<Vec<StoreEvent<<Self::Aggregate as crate::Aggregate>::Event>>, Self::Error>;

    /// Publish multiple events on the configured events buses.
    async fn publish(&self, store_events: &[StoreEvent<<Self::Aggregate as crate::Aggregate>::Event>]);

    /// Delete all events from events store related to given `aggregate_id`.
    ///
    /// Moreover it should delete all the read side projections triggered by event handlers.
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Self::Error>;
}

/// Blanket implementation making an [`EventStore`] every (smart) pointer to an [`EventStore`],
/// e.g. `&Store`, `Box<Store>`, `Arc<Store>`.
/// This is particularly useful when there's the need in your codebase to have a generic [`EventStore`].
#[async_trait]
impl<A, E, T, S> EventStore for T
where
    A: crate::Aggregate,
    A::Event: Send + Sync,
    A::State: Send,
    E: std::error::Error,
    S: EventStore<Aggregate = A, Error = E> + ?Sized,
    T: Deref<Target = S> + Sync,
    for<'a> A::Event: 'a,
{
    type Aggregate = A;
    type Error = E;

    /// Deref call to [`EventStore::lock`].
    async fn lock(&self, aggregate_id: Uuid) -> Result<EventStoreLockGuard, Self::Error> {
        self.deref().lock(aggregate_id).await
    }

    /// Deref call to [`EventStore::by_aggregate_id`].
    async fn by_aggregate_id(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<StoreEvent<<Self::Aggregate as crate::Aggregate>::Event>>, Self::Error> {
        self.deref().by_aggregate_id(aggregate_id).await
    }

    /// Deref call to [`EventStore::persist`].
    async fn persist(
        &self,
        aggregate_state: &mut AggregateState<<Self::Aggregate as crate::Aggregate>::State>,
        events: Vec<<Self::Aggregate as crate::Aggregate>::Event>,
    ) -> Result<Vec<StoreEvent<<Self::Aggregate as crate::Aggregate>::Event>>, Self::Error> {
        self.deref().persist(aggregate_state, events).await
    }

    /// Deref call to [`EventStore::publish`].
    async fn publish(&self, events: &[StoreEvent<<Self::Aggregate as crate::Aggregate>::Event>]) {
        self.deref().publish(events).await
    }

    /// Deref call to [`EventStore::delete`].
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Self::Error> {
        self.deref().delete(aggregate_id).await
    }
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
    /// The version of the event.
    pub version: Option<i32>,
}

impl<Event> StoreEvent<Event> {
    /// Returns the sequence number of the event, within its specific aggregate instance.
    pub const fn sequence_number(&self) -> &SequenceNumber {
        &self.sequence_number
    }

    /// Returns the original, emitted, event.
    pub const fn payload(&self) -> &Event {
        &self.payload
    }
}
