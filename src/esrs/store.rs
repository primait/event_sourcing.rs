use std::ops::Deref;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::types::SequenceNumber;
use crate::{Aggregate, AggregateManager, AggregateState};

/// Marker trait for every EventStoreLockGuard.
///
/// Implementors should unlock concurrent access to the guarded resource, when dropped.
pub trait UnlockOnDrop: Send + Sync + 'static {}

/// Lock guard preventing concurrent access to a resource.
///
/// The lock is released when this guard is dropped.
pub struct EventStoreLockGuard(Box<dyn UnlockOnDrop>);

impl EventStoreLockGuard {
    /// Creates a new instance from any UnlockOnDrop.
    #[must_use]
    pub fn new(lock: impl UnlockOnDrop) -> Self {
        Self(Box::new(lock))
    }
}

/// An EventStore is responsible for persisting events that an aggregate emits into a database, and loading the events
/// that represent an aggregate's history from the database.
#[async_trait]
pub trait EventStore {
    type Manager: AggregateManager;

    /// Acquires a lock for the given aggregate, or waits for outstanding guards to be released.
    ///
    /// Used to prevent concurrent access to the aggregate state.
    /// Note that any process which does *not* `lock` will get immediate (possibly shared!) access.
    /// ALL accesses (regardless of this guard) are subject to the usual optimistic locking strategy on write.
    async fn lock(&self, aggregate_id: Uuid) -> Result<EventStoreLockGuard, <Self::Manager as Aggregate>::Error>;

    /// Loads the events that an aggregate instance has emitted in the past.
    async fn by_aggregate_id(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<StoreEvent<<Self::Manager as Aggregate>::Event>>, <Self::Manager as Aggregate>::Error>;

    /// Persists multiple events into the database. This should be done in a single transaction - either
    /// all the events are persisted correctly, or none are.
    ///
    /// Persisting events may additionally trigger configured event handlers (transactional and non-transactional).
    async fn persist(
        &self,
        aggregate_state: &mut AggregateState<<Self::Manager as Aggregate>::State>,
        events: Vec<<Self::Manager as Aggregate>::Event>,
    ) -> Result<Vec<StoreEvent<<Self::Manager as Aggregate>::Event>>, <Self::Manager as Aggregate>::Error>;

    /// Delete all events from events store related to given `aggregate_id`.
    ///
    /// Moreover it should delete all the read side projections triggered by event handlers.
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), <Self::Manager as Aggregate>::Error>;
}

/// Default generic implementation for every type implementing [`Deref`] where its `Target` is a
/// `dyn` [`EventStore`]. This is particularly useful when there's the need in your codebase to have
/// a generic [`EventStore`] inside your [`Aggregate`].
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
impl<M, T> EventStore for T
where
    T: Deref<Target = dyn EventStore<Manager = M> + Send + Sync> + Sync,
    M: AggregateManager,
    <M as Aggregate>::Event: 'static,
{
    type Manager = M;

    async fn lock(&self, aggregate_id: Uuid) -> Result<EventStoreLockGuard, <Self::Manager as Aggregate>::Error> {
        self.deref().lock(aggregate_id).await
    }

    async fn by_aggregate_id(
        &self,
        aggregate_id: Uuid,
    ) -> Result<Vec<StoreEvent<<Self::Manager as Aggregate>::Event>>, <Self::Manager as Aggregate>::Error> {
        self.deref().by_aggregate_id(aggregate_id).await
    }

    async fn persist(
        &self,
        aggregate_state: &mut AggregateState<<Self::Manager as Aggregate>::State>,
        events: Vec<<Self::Manager as Aggregate>::Event>,
    ) -> Result<Vec<StoreEvent<<Self::Manager as Aggregate>::Event>>, <Self::Manager as Aggregate>::Error> {
        self.deref().persist(aggregate_state, events).await
    }

    async fn delete(&self, aggregate_id: Uuid) -> Result<(), <Self::Manager as Aggregate>::Error> {
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
}

impl<Event> StoreEvent<Event> {
    pub const fn sequence_number(&self) -> &SequenceNumber {
        &self.sequence_number
    }

    pub const fn payload(&self) -> &Event {
        &self.payload
    }
}
