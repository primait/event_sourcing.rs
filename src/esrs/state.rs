use uuid::Uuid;

use crate::esrs::store::EventStoreLockGuard;
use crate::esrs::store::StoreEvent;
use crate::types::SequenceNumber;

/// The internal state for an Aggregate.
/// It contains:
/// - an id uniquely representing the aggregate,
/// - an incremental sequence number,
/// - a lock representing the atomicity of the access to the aggregate,
/// - a state defined by the user of this library.
pub struct AggregateState<S> {
    id: Uuid,
    sequence_number: SequenceNumber,
    lock: Option<EventStoreLockGuard>,
    inner: S,
}

impl<S: std::fmt::Debug> std::fmt::Debug for AggregateState<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AggregateState")
            .field("id", &self.id)
            .field("sequence_number", &self.sequence_number)
            .field("lock", &self.lock.is_some())
            .field("inner", &self.inner)
            .finish()
    }
}

/// Default implementation for [`AggregateState`]
impl<S: Default> Default for AggregateState<S> {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Default> AggregateState<S> {
    /// Creates a new instance of an [`AggregateState`] with a new unique id.
    pub fn new() -> Self {
        Self {
            id: Uuid::new_v4(),
            inner: Default::default(),
            sequence_number: 0,
            lock: None,
        }
    }

    /// Creates a new instance of an [`AggregateState`] with the given aggregate id.
    ///
    /// This should be used almost exclusively when loading by aggregate id yields nothing,
    /// and this becomes the brand new aggregate state for that id.
    ///
    /// Other uses are strongly discouraged, since the id could be already existing
    /// and a clash might happen when persisting events.
    pub fn with_id(id: impl Into<Uuid>) -> Self {
        Self {
            id: id.into(),
            inner: Default::default(),
            sequence_number: 0,
            lock: None,
        }
    }

    /// Consumes the aggregate state and generates a new one with the events applied to it,
    /// as dictated by `apply_event`.
    pub fn apply_store_events<T, F>(self, store_events: Vec<StoreEvent<T>>, apply_event: F) -> Self
    where
        F: Fn(S, T) -> S,
    {
        store_events.into_iter().fold(self, |state, store_event| {
            let sequence_number = *store_event.sequence_number();
            let inner = apply_event(state.inner, store_event.payload);

            Self {
                sequence_number,
                inner,
                ..state
            }
        })
    }

    /// Returns an Uuid representing the aggregate id.
    pub const fn id(&self) -> &Uuid {
        &self.id
    }

    /// Returns the internal state.
    pub const fn inner(&self) -> &S {
        &self.inner
    }

    /// Consumes self and extracts the internal state.
    pub fn into_inner(self) -> S {
        self.inner
    }

    /// Returns the internal sequence number.
    pub const fn sequence_number(&self) -> &SequenceNumber {
        &self.sequence_number
    }

    /// Computes the internal sequence number incremented by 1.
    pub fn next_sequence_number(&self) -> SequenceNumber {
        self.sequence_number + 1
    }

    /// Inserts the lock guard into self, replacing any current one.
    pub fn set_lock(&mut self, guard: EventStoreLockGuard) {
        self.lock = Some(guard);
    }

    /// Extracts the lock from self, leaving nothing behind.
    pub fn take_lock(&mut self) -> Option<EventStoreLockGuard> {
        self.lock.take()
    }
}
