use async_trait::async_trait;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::esrs::store::StoreEvent;

/// Projector trait that takes a Postgres transaction in order to create a read model
#[async_trait]
pub trait Projector<Aggregate>
where
    Aggregate: crate::aggregate::Aggregate,
{
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    /// TODO: doc for pg connection
    async fn project(
        &self,
        event: &StoreEvent<Aggregate::Event>,
        connection: &mut PgConnection,
    ) -> Result<(), Aggregate::Error>;

    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    /// TODO: doc for pg connection
    async fn delete(&self, aggregate_id: Uuid, connection: &mut PgConnection) -> Result<(), Aggregate::Error>;
}
