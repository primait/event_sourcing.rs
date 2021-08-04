use std::future::Future;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::esrs::SequenceNumber;

#[async_trait]
pub trait EventStore<Event: Serialize + DeserializeOwned + Send + Sync, Error> {
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Event>>, Error>;

    async fn persist(
        &self,
        aggregate_id: Uuid,
        event: Event,
        sequence_number: SequenceNumber,
    ) -> Result<StoreEvent<Event>, Error>;

    async fn close(&self);
}

pub trait ProjectorStore<Event: Serialize + DeserializeOwned + Send + Sync, Executor, Error> {
    fn project_event<'a>(
        &'a self,
        store_event: &'a StoreEvent<Event>,
        executor: &'a mut Executor,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>
    where
        Self: Sync + 'a;
}

#[async_trait]
pub trait EraserStore<Event: Serialize + DeserializeOwned + Send + Sync, Error> {
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Error>;
}

pub struct StoreEvent<Event: Serialize + DeserializeOwned + Send + Sync> {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Event,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
}

impl<Event: Serialize + DeserializeOwned + Send + Sync> StoreEvent<Event> {
    pub fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub fn payload(&self) -> &Event {
        &self.payload
    }
}
