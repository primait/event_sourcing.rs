use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::esrs::SequenceNumber;

/// An EventStore is responsible for persisting events that an aggregate emits into a database, and loading the events
/// that represent an aggregate's history from the database.
#[async_trait]
pub trait EventStore<Event, Error>
where
    Event: Serialize + DeserializeOwned,
{
    /// Loads the events that an aggregate instance has emitted in the past.
    async fn by_aggregate_id(&self, aggregate_id: Uuid) -> Result<Vec<StoreEvent<Event>>, Error>;

    /// Persists multiple events into the database. This should be done in a single transaction - either
    /// all the events are persisted correctly, or none are.
    ///
    /// Persisting events may additionally trigger configured Projectors.
    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Event>>, Error>;

    // TODO: doc
    async fn delete_by_aggregate_id(&self, aggregate_id: Uuid) -> Result<(), Error>;
}

/// A StoreEvent contains the payload (the original event) alongside the event's metadata.
pub struct StoreEvent<Event: DeserializeOwned> {
    /// Uniquely identifies an event among all events emitted from all aggregates.
    pub id: Uuid,
    /// The aggregate instance that emitted the event.
    pub aggregate_id: Uuid,
    /// The original, emitted, event.
    pub payload: Event,
    /// The timestamp of when the event is persisted.
    pub occurred_on: DateTime<Utc>,
    /// The sequence number of the event, within its specific aggregate instance.
    pub sequence_number: SequenceNumber,
}

impl<Event: DeserializeOwned> StoreEvent<Event> {
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub const fn payload(&self) -> &Event {
        &self.payload
    }
}
