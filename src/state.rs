use uuid::Uuid;

use crate::SequenceNumber;

pub struct AggregateState<S: Default> {
    pub id: Uuid,
    pub sequence_number: SequenceNumber,
    pub inner: S,
}

impl<S: Default> Default for AggregateState<S> {
    fn default() -> Self {
        Self {
            id: Uuid::new_v4(),
            sequence_number: 0,
            inner: Default::default(),
        }
    }
}

impl<S: Default> AggregateState<S> {
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

    pub fn get_sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub fn set_sequence_number(self, sequence_number: SequenceNumber) -> Self {
        Self {
            sequence_number,
            ..self
        }
    }

    pub fn next_sequence_number(&self) -> SequenceNumber {
        &self.sequence_number + 1
    }
}
