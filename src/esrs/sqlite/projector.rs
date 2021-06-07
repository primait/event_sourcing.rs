use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::Sqlite;
use uuid::Uuid;

use crate::esrs::pool::Transaction;
use crate::esrs::store::StoreEvent;

/// Projector trait that takes a Sqlite transaction in order to create a read model
#[async_trait]
pub trait SqliteProjector<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    async fn project<'c>(
        &self,
        event: &StoreEvent<Event>,
        transaction: &mut Transaction<'c, Sqlite>,
    ) -> Result<(), Error>;
}

/// Projector trait that takes a Sqlite transaction in order to delete a read model
#[async_trait]
pub trait SqliteProjectorEraser<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error>:
    SqliteProjector<Event, Error>
{
    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    async fn delete<'c>(&self, aggregate_id: Uuid, transaction: &mut Transaction<'c, Sqlite>) -> Result<(), Error>;
}
