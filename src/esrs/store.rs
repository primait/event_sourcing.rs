use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
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
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Event>>, Error>;

    /// Persists multiple events into the database.  This should be done in a single transaction - either
    /// all the events are persisted correctly, or none are.
    ///
    /// Persisting events may additionally trigger configured Projectors.
    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Event>>, Error>;

    // /// Run any policies attached to this store against a set of events.
    // /// This should be called only after the events have successfully been persisted in the store.
    // async fn run_policies(&self, events: &[StoreEvent<Event>]) -> Result<(), Error>;

    async fn close(&self);

    fn get_all(&self) -> BoxStream<Result<StoreEvent<Event>, Error>>;
}

/// A ProjectorStore is responsible for projecting an event (that has been persisted to the database) into a
/// form that is better suited to being read by other parts of the application.
#[async_trait]
pub trait ProjectorStore<Executor, Event, Error>
where
    Event: DeserializeOwned,
{
    async fn project_event(&self, store_event: &StoreEvent<Event>, executor: &mut Executor) -> Result<(), Error>;
}

/// An EraserStore is responsible for wiping an aggregate instance from history: it should delete the
/// aggregate instance, along with all of its events, or fail.
#[async_trait]
pub trait EraserStore<Event, Error> {
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Error>;
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
