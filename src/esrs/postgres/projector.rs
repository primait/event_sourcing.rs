use async_trait::async_trait;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{AggregateManager, StoreEvent};

/// This trait is used to implement a `Projector`. A projector is intended to be an entity where to
/// create, update and delete a read side. Every projector should be responsible to update a single
/// read model.
#[async_trait]
pub trait Projector<Manager>: Sync
where
    Manager: AggregateManager,
{
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    ///
    /// Note: in actual implementation the second parameter is an &mut PgConnection. In further releases
    /// of sqlx package this could be changed. At this time the connection could be a simple connection
    /// acquired by a pool or a deref of a transaction.
    async fn project(
        &self,
        event: &StoreEvent<Manager::Event>,
        connection: &mut PgConnection,
    ) -> Result<(), Manager::Error>;

    /// Delete the read model entry. It is here because of the eventual need of delete an entire
    /// aggregate.
    ///
    /// Default implementation *does nothing* and always returns an Ok. Override this function to
    /// implement deletion behaviour for custom projections.
    ///
    /// Note: in actual implementation the second parameter is an &mut PgConnection. In further releases
    /// of sqlx package this could be changed. At this time the connection could be a simple connection
    /// acquired by a pool or a deref of a transaction.
    async fn delete(&self, _aggregate_id: Uuid, _connection: &mut PgConnection) -> Result<(), Manager::Error> {
        Ok(())
    }
}
