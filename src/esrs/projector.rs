use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::Transaction;
use uuid::Uuid;

use crate::esrs::store::StoreEvent;

/// Projector trait that takes an executor in order to create a read model
#[async_trait]
pub trait Projector<Database: sqlx::Database, Event: Serialize + DeserializeOwned + Send + Sync, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    async fn project(&self, event: &StoreEvent<Event>, executor: &mut Transaction<Database>) -> Result<(), Error>;
}

/// Projector trait that takes an executor in order to delete a read model
#[async_trait]
pub trait ProjectorEraser<Database: sqlx::Database, Event: Serialize + DeserializeOwned + Send + Sync, Error>:
    Projector<Database, Event, Error>
{
    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    async fn delete(&self, aggregate_id: Uuid, executor: &mut Transaction<Database>) -> Result<(), Error>;
}
