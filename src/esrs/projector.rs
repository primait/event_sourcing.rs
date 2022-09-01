use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::esrs::store::StoreEvent;

/// Projector trait that takes an executor in order to create a read model
#[async_trait]
pub trait Projector<Executor, Event: Serialize + DeserializeOwned + Send + Sync, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    async fn project(&self, event: &StoreEvent<Event>, executor: &mut Executor) -> Result<(), Error>;
}

/// Projector trait that takes an executor in order to delete a read model
#[async_trait]
pub trait ProjectorEraser<Executor, Event: Serialize + DeserializeOwned + Send + Sync, Error>:
    Projector<Executor, Event, Error>
{
    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    async fn delete(&self, aggregate_id: Uuid, executor: &mut Executor) -> Result<(), Error>;
}
