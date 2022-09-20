use async_trait::async_trait;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::aggregate::AggregateManager;
use crate::store::StoreEvent;

/// Projector trait that takes a Postgres transaction in order to create a read model
#[async_trait]
pub trait Projector<Manager>
where
    Self: ProjectorClone<Manager>,
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
    /// Note: in actual implementation the second parameter is an &mut PgConnection. In further releases
    /// of sqlx package this could be changed. At this time the connection could be a simple connection
    /// acquired by a pool or a deref of a transaction.
    async fn delete(&self, aggregate_id: Uuid, connection: &mut PgConnection) -> Result<(), Manager::Error>;
}

pub trait ProjectorClone<Manager>
where
    Manager: AggregateManager,
{
    fn clone_box(&self) -> Box<dyn Projector<Manager> + Send + Sync>;
}

impl<T, Manager> ProjectorClone<Manager> for T
where
    T: 'static + Projector<Manager> + Clone + Send + Sync,
    Manager: AggregateManager,
{
    fn clone_box(&self) -> Box<dyn Projector<Manager> + Send + Sync> {
        Box::new(self.clone())
    }
}

impl<Manager> Clone for Box<dyn Projector<Manager> + Send + Sync>
where
    Manager: AggregateManager,
{
    fn clone(&self) -> Box<dyn Projector<Manager> + Send + Sync> {
        self.clone_box()
    }
}
