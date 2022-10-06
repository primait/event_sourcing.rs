use uuid::Uuid;

use crate::types::SequenceNumber;

/// The internal state for an Aggregate.
/// It contains and id representing the aggregate id, an incremental sequence number and a state
/// defined by the user of this library.
#[derive(Clone)]
pub struct AggregateState<S> {
    pub(crate) id: Uuid,
    pub(crate) sequence_number: SequenceNumber,
    pub(crate) inner: S,
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
    #[must_use]
    pub fn new(id: impl Into<Uuid>) -> Self {
        Self {
            id,
            inner: Default::default(),
            sequence_number: 0,
        }
    }

    /// Returns an Uuid representing the aggregate id
    pub const fn id(&self) -> &Uuid {
        &self.id
    }

    /// Returns the internal state
    pub const fn inner(&self) -> &S {
        &self.inner
    }

    /// Returns the internal sequence number incremented by 1.
    pub const fn next_sequence_number(&self) -> SequenceNumber {
        self.sequence_number + 1
    }
}
