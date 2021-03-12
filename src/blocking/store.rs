use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::SequenceNumber;

#[cfg(feature = "postgresql")]
pub mod postgres;

pub trait EventStore<Event: Serialize + DeserializeOwned + Clone, Error> {
    fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Event>>, Error>;

    fn persist(
        &self,
        aggregate_id: Uuid,
        event: Event,
        sequence_number: SequenceNumber,
    ) -> Result<StoreEvent<Event>, Error>;

    fn rebuild_event(&self, store_event: &StoreEvent<Event>) -> Result<(), Error>;

    fn close(&self);
}

#[derive(Clone)]
pub struct StoreEvent<Event: Serialize + DeserializeOwned + Clone> {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Event,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
}

impl<Event: Serialize + DeserializeOwned + Clone> StoreEvent<Event> {
    pub fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub fn payload(&self) -> &Event {
        &self.payload
    }
}
