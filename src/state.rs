use uuid::Uuid;

use crate::SequenceNumber;

#[derive(Clone)]
pub struct AggregateState<S: Default + Clone> {
    id: Uuid,
    sequence_number: SequenceNumber,
    inner: S,
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

    pub fn id(&self) -> Uuid {
        self.id
    }

    pub fn inner(&self) -> &S {
        &self.inner
    }

    pub fn inner_mut(&mut self) -> &mut S {
        &mut self.inner
    }

    pub fn set_inner(&mut self, s: S) -> &mut Self {
        self.inner = s;
        self
    }

    pub fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub(crate) fn set_sequence_number(&mut self, sequence_number: SequenceNumber) -> &mut Self {
        self.sequence_number = sequence_number;
        self
    }

    pub(crate) fn incr_sequence_number(&mut self) -> &mut Self {
        self.sequence_number += 1;
        self
    }
}
