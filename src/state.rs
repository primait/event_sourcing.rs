use uuid::Uuid;

use crate::SequenceNumber;

#[derive(Clone)]
pub struct AggregateState<S: Default + Clone> {
    pub(crate) id: Uuid,
    pub(crate) sequence_number: SequenceNumber,
    pub(crate) inner: S,
}

impl<S: Default + Clone> Default for AggregateState<S> {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            sequence_number: 0,
            inner: Default::default(),
        }
    }
}

impl<S: Default + Clone> AggregateState<S> {
    pub fn new(id: Uuid) -> Self {
        Self {
            id,
            inner: Default::default(),
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
