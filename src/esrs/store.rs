use std::future::Future;
use std::pin::Pin;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::esrs::SequenceNumber;
use crate::policy::Policy;
use crate::projector::Projector;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "postgres")]
mod setup;

/// An EventStore is responsible for persisting events that an aggregate emits into a database, and loading the events
/// that represent an aggregate's history from the database.
pub trait EventStore<Event: Serialize + DeserializeOwned + Send + Sync, Error> {
    /// Loads the events that an aggregate instance has emitted in the past.
    fn by_aggregate_id<'a>(
        &'a self,
        id: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StoreEvent<Event>>, Error>> + Send + 'a>>;

    /// Persists multiple events into the database.  This should be done in a single transaction - either
    /// all the events are persisted correctly, or none are.
    ///
    /// Persisting events may additionally trigger configured Projectors.
    fn persist<'a>(
        &'a self,
        aggregate_id: Uuid,
        events: Vec<Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Pin<Box<dyn Future<Output = Result<Vec<StoreEvent<Event>>, Error>> + Send + 'a>>;

    fn projectors(&self) -> &Vec<Box<dyn Projector<Event, Error> + Send + Sync>>;

    /// Run any projector attached to this store against a set of events.
    fn project_events<'a>(
        &'a self,
        store_event: &'a [StoreEvent<Event>],
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

    fn policies(&self) -> &Vec<Box<dyn Policy<Event, Error> + Send + Sync>>;

    /// Run any policies attached to this store against a set of events.
    /// This should be called only after the events have successfully been persisted in the store.
    fn run_policies<'a>(
        &'a self,
        events: &'a [StoreEvent<Event>],
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

    fn delete_by_aggregate_id<'a>(
        &'a self,
        aggregate_id: Uuid,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

    fn close<'a>(&'a self) -> Pin<Box<dyn Future<Output = ()> + Send + 'a>>;
}

/// A StoreEvent contains the payload (the original event) alongside the event's metadata.
pub struct StoreEvent<Event: Serialize + DeserializeOwned + Send + Sync> {
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

impl<Event: Serialize + DeserializeOwned + Send + Sync> StoreEvent<Event> {
    pub const fn sequence_number(&self) -> SequenceNumber {
        self.sequence_number
    }

    pub const fn payload(&self) -> &Event {
        &self.payload
    }
}
