use std::fmt::Debug;

use uuid::Uuid;

use crate::esrs::SequenceNumber;

#[derive(Debug, Clone)]
pub struct AggregateState<S: Default + Debug + Clone> {
    pub(crate) id: Uuid,
    pub(crate) sequence_number: SequenceNumber,
    pub(crate) inner: S,
}

impl<S: Default + Debug + Clone> Default for AggregateState<S> {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            sequence_number: 0,
            inner: Default::default(),
        }
    }
}

impl<S: Default + Debug + Clone> AggregateState<S> {
    #[must_use]
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            inner: Default::default(),
            sequence_number: 0,
        }
    }

    pub fn new_with_state(id: Uuid, inner: S) -> Self {
        Self {
            id,
            inner,
            sequence_number: 0,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }
}
