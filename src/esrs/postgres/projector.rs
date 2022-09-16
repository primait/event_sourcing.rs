use async_trait::async_trait;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::aggregate;
use crate::store::StoreEvent;

/// Projector trait that takes a Postgres transaction in order to create a read model
#[async_trait]
pub trait Projector<Aggregate>
where
    Aggregate: aggregate::Aggregate,
{
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    ///
    /// Note: in actual implementation the second parameter is an &mut PgConnection. In further releases
    /// of sqlx package this could be changed. At this time the connection could be a simple connection
    /// acquired by a pool or a deref of a transaction.
    async fn project(
        &self,
        event: &StoreEvent<Aggregate::Event>,
        connection: &mut PgConnection,
    ) -> Result<(), Aggregate::Error>;

    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    ///
    /// Note: in actual implementation the second parameter is an &mut PgConnection. In further releases
    /// of sqlx package this could be changed. At this time the connection could be a simple connection
    /// acquired by a pool or a deref of a transaction.
    async fn delete(&self, aggregate_id: Uuid, connection: &mut PgConnection) -> Result<(), Aggregate::Error>;
}
