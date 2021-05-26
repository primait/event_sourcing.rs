use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Postgres, Transaction};
use uuid::Uuid;

use crate::esrs::store::StoreEvent;

#[async_trait]
pub trait PgProjector<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    async fn project<'c>(
        &self,
        event: &StoreEvent<Event>,
        transaction: &mut Transaction<'c, Postgres>,
    ) -> Result<(), Error>;

    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    async fn delete<'c>(&self, aggregate_id: Uuid, transaction: &mut Transaction<'c, Postgres>) -> Result<(), Error>;
}
