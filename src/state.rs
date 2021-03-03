use uuid::Uuid;

use crate::SequenceNumber;

#[derive(Clone)]
pub struct AggregateState<S: Default + Clone> {
    pub id: Uuid,
    pub sequence_number: SequenceNumber,
    pub inner: S,
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

    pub fn new_with(id: Uuid, state: S) -> Self {
        Self {
            id,
            inner: state,
            sequence_number: 0,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn set_inner(&mut self, inner: S) -> &Self {
        self.inner = inner;
        self
    }

    pub fn get_sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub fn set_sequence_number(self, sequence_number: SequenceNumber) -> Self {
        Self {
            sequence_number,
            ..self
        }
    }

    pub fn incr_sequence_number(self) -> Self {
        Self {
            sequence_number: self.sequence_number + 1,
            ..self
        }
    }
}
