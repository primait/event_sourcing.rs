use uuid::Uuid;

use crate::esrs::store::EventStoreLockGuard;
use crate::types::SequenceNumber;

/// The internal state for an Aggregate.
/// It contains:
/// - an id uniquely representing the aggregate,
/// - an incremental sequence number,
/// - a lock representing the atomicity of the access to the aggregate,
/// - a state defined by the user of this library.
pub struct AggregateState<S> {
    pub(crate) id: Uuid,
    pub(crate) sequence_number: SequenceNumber,
    pub(crate) lock: Option<EventStoreLockGuard>,
    pub(crate) inner: S,
}

impl<S: Clone> Clone for AggregateState<S> {
    fn clone(&self) -> Self {
        Self {
            id: self.id,
            sequence_number: self.sequence_number,
            inner: self.inner.clone(),
            lock: None,
        }
    }
}

impl<S: std::fmt::Debug> std::fmt::Debug for AggregateState<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AggregateState")
            .field("id", &self.id)
            .field("sequence_number", &self.sequence_number)
            .field("inner", &self.inner)
            .field("lock", &self.lock.is_some())
            .finish()
    }
}

/// Default implementation for [`AggregateState`]
impl<S: Default> Default for AggregateState<S> {
    fn default() -> Self {
        Self::new(Uuid::new_v4())
    }
}

impl<S: Default> AggregateState<S> {
    /// Creates a new instance of an [`AggregateState`] with the given aggregate id. The use of this
    /// is discouraged being that that aggregate id could be already existing and a clash of ids
    /// might happen.
    ///
    /// Prefer [Default] implementation.
    pub fn new(id: impl Into<Uuid>) -> Self {
        Self {
            id: id.into(),
            inner: Default::default(),
            sequence_number: 0,
            lock: None,
        }
    }

    /// Returns an Uuid representing the aggregate id.
    pub const fn id(&self) -> &Uuid {
        &self.id
    }

    /// Returns the internal state.
    pub const fn inner(&self) -> &S {
        &self.inner
    }

    /// Returns the internal sequence number incremented by 1.
    pub const fn next_sequence_number(&self) -> SequenceNumber {
        self.sequence_number + 1
    }

    /// Insert the lock guard into self, replacing any current one.
    pub fn set_lock(&mut self, guard: EventStoreLockGuard) {
        self.lock = Some(guard);
    }

    /// Extract the lock from self, leaving nothing behind.
    pub fn take_lock(&mut self) -> Option<EventStoreLockGuard> {
        self.lock.take()
    }
}
