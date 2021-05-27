use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::esrs::store::StoreEvent;

/// Projector trait that takes a Postgres transaction in order to create a read model
#[async_trait]
pub trait PgProjector<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    async fn project<'c>(
        &self,
        event: &StoreEvent<Event>,
        transaction: &mut Transaction<'c, Postgres>,
    ) -> Result<(), Error>;
}

/// Projector trait that takes a Postgres transaction in order to delete a read model
#[async_trait]
pub trait PgProjectorEraser<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error>:
    PgProjector<Event, Error>
{
    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    async fn delete<'c>(&self, aggregate_id: Uuid, transaction: &mut Transaction<'c, Postgres>) -> Result<(), Error>;
}
