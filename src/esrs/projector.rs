use async_trait::async_trait;
use serde::de::DeserializeOwned;
use sqlx::Transaction;
use uuid::Uuid;

use crate::esrs::store::StoreEvent;

/// Projector trait that takes a Postgres transaction in order to create a read model
#[async_trait]
pub trait Projector<Database, Event, Error>
where
    Database: sqlx::Database,
    Event: DeserializeOwned,
{
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    async fn project(
        &self,
        event: &StoreEvent<Event>,
        transaction: &mut Transaction<'_, Database>,
    ) -> Result<(), Error>;

    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    async fn delete(&self, aggregate_id: Uuid, transaction: &mut Transaction<'_, Database>) -> Result<(), Error>;
}
