use async_trait::async_trait;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::{AggregateManager, StoreEvent};

/// This enum is used to instruct via [`Projector::persistence`] function which guarantees to have
/// while projecting an event in the read side.
/// - [`ProjectorPersistence::Mandatory`] means that the projected data will be always available in the read
///   side. In the actual default store implementation it implies that if a mandatory persistent
///   projector fails the event will not be stored in the event store and the transaction rollbacks.
/// - [`ProjectorPersistence::Fallible`] means that there are no guarantees for the projected data to be
///   persisted in the read side. In the actual default store implementation it implies that if an
///   fallible persistent projector fails that event is stored anyway but nothing will be persisted
///   in the read side. If no other projector fails the event will be stored in the event store with
///   all the other projections and the transaction will be committed.
pub enum ProjectorPersistence {
    Mandatory,
    Fallible,
}

impl AsRef<str> for ProjectorPersistence {
    fn as_ref(&self) -> &str {
        match self {
            Self::Mandatory => "mandatory",
            Self::Fallible => "fallible",
        }
    }
}

/// This trait is used to implement a `Projector`. A projector is intended to be an entity where to
/// create, update and delete a read side. Every projector should be responsible to update a single
/// read model.
#[async_trait]
pub trait Projector<Manager>: Sync
where
    Manager: AggregateManager,
{
    /// This function could be used to instruct the [`Projector`] about its the
    /// [`ProjectorPersistence`] level.
    ///
    /// It has a default implementation that returns [`ProjectorPersistence::Mandatory`].
    /// Override this function to change its [`ProjectorPersistence`] level.
    fn persistence(&self) -> ProjectorPersistence {
        ProjectorPersistence::Mandatory
    }

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

    /// The name of the projector. By default, this is the type name of the projector,
    /// but it can be overridden to provide a custom name. This name is used as
    /// part of tracing spans, to identify the projector being run.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
