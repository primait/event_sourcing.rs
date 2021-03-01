use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

#[cfg(test)]
use mockall::automock;

use crate::SequenceNumber;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait EventStore<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Event>>, Error>;

    async fn persist(
        &self,
        aggregate_id: Uuid,
        event: Event,
        sequence_number: SequenceNumber,
    ) -> Result<StoreEvent<Event>, Error>;

    async fn rebuild_event(&self, store_event: &StoreEvent<Event>) -> Result<(), Error>;
}

#[derive(Clone)]
pub struct StoreEvent<Event: Serialize + DeserializeOwned + Clone + Send + Sync> {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Event,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub sequence_number: SequenceNumber,
}

impl<Event: Serialize + DeserializeOwned + Clone + Send + Sync> StoreEvent<Event> {
    pub fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub fn payload(&self) -> &Event {
        &self.payload
    }
}
