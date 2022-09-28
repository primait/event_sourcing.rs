use uuid::Uuid;

use crate::types::SequenceNumber;

#[derive(Clone)]
pub struct AggregateState<S> {
    pub(crate) id: Uuid,
    pub(crate) sequence_number: SequenceNumber,
    pub(crate) inner: S,
}

impl<S: Default> Default for AggregateState<S> {
    fn default() -> Self {
        Self::new(Uuid::new_v4())
    }
}

impl<S: Default> AggregateState<S> {
    #[must_use]
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            inner: Default::default(),
            sequence_number: 0,
        }
    }

    pub const fn id(&self) -> &Uuid {
        &self.id
    }

    pub const fn inner(&self) -> &S {
        &self.inner
    }

    pub const fn next_sequence_number(&self) -> SequenceNumber {
        self.sequence_number + 1
    }
}
