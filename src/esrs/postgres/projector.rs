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

/// This trait ensures that every `Box<dyn Projector<T>>` exposes a `clone_box` function, used later in
/// the other traits.
///
/// In the [`Projector`] this bound is useful in order to let it be cloneable. Being cloneable is
/// mandatory in order to let the implementation of an `EventStore` be cloneable.
///
/// Keep in mind that while implementing `Projector<T>` for a struct it's needed for that struct to be
/// cloneable.
pub trait ProjectorClone<Manager>
where
    Manager: AggregateManager,
{
    fn clone_box(&self) -> Box<dyn Projector<Manager> + Send + Sync>;
}

/// This trait create a default implementation of `ProjectorClone` for every `T` where `T` is a `dyn Projector`,
/// to avoid the implementor of a [`Projector`] have to implement this trait and having this functionality
/// by default.
impl<T, Manager> ProjectorClone<Manager> for T
where
    T: 'static + Projector<Manager> + Clone + Send + Sync,
    Manager: AggregateManager,
{
    fn clone_box(&self) -> Box<dyn Projector<Manager> + Send + Sync> {
        Box::new(self.clone())
    }
}

/// This trait implements [`Clone`] for every `Box<dyn Projector<T>>` calling [`ProjectorClone`]
/// `clone_box` function.
impl<Manager> Clone for Box<dyn Projector<Manager> + Send + Sync>
where
    Manager: AggregateManager,
{
    fn clone(&self) -> Box<dyn Projector<Manager> + Send + Sync> {
        self.clone_box()
    }
}
