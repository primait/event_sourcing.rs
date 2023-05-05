use async_trait::async_trait;
use std::ops::Deref;
use uuid::Uuid;

use crate::{Aggregate, StoreEvent};

/// This trait is used to implement an `EventHandler`. An event handler is intended to be an entity
/// where to create, update and delete a read side and perform side effects.
///
/// The main purpose of an `EventHandler` is to have an eventually persistent processor.
#[async_trait]
pub trait EventHandler<A>: Sync
where
    A: Aggregate,
{
    /// Handle an event and perform an action. This action could be over a read model or a side-effect.
    /// All the errors should be handled from within the `EventHandler` and shouldn't panic.
    async fn handle(&self, event: &StoreEvent<A::Event>);

    /// Perform a deletion of a resource using the given aggregate_id.
    async fn delete(&self, _aggregate_id: Uuid) {}

    /// The name of the event handler. By default, this is the type name of the event handler,
    /// but it can be overridden to provide a custom name. This name is used as
    /// part of tracing spans, to identify the event handler being run.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

#[async_trait]
impl<A, Q, T> EventHandler<A> for T
where
    A: Aggregate,
    A::Event: Send + Sync,
    Q: EventHandler<A>,
    T: Deref<Target = Q> + Send + Sync,
{
    /// Deref call to [`EventHandler::handle`].
    async fn handle(&self, event: &StoreEvent<A::Event>) {
        self.deref().handle(event).await;
    }
}

/// This trait is used to implement a `TransactionalEventHandler`. A transactional event handler is
/// intended to be an entity where to create, update and delete a read side. No side effects must be
/// performed inside of this kind on handler.
///
/// An `handle` operation will result in a _deadlock_ if the implementation of this trait is used to
/// apply an event on an [`Aggregate`].
#[async_trait]
pub trait TransactionalEventHandler<A, E>: Sync
where
    A: Aggregate,
{
    /// Handle an event in a transactional fashion and perform a read side crate, update or delete.
    /// If an error is returned the transaction will be aborted and the handling of a command by an
    /// aggregate will return an error.
    async fn handle(&self, event: &StoreEvent<A::Event>, executor: &mut E) -> Result<(), A::Error>;

    /// Perform a deletion of a read side projection using the given aggregate_id.
    async fn delete(&self, _aggregate_id: Uuid, _executor: &mut E) -> Result<(), A::Error> {
        Ok(())
    }

    /// The name of the event handler. By default, this is the type name of the event handler,
    /// but it can be overridden to provide a custom name. This name is used as
    /// part of tracing spans, to identify the event handler being run.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

#[async_trait]
impl<A, E, Q, T> TransactionalEventHandler<A, E> for T
where
    A: Aggregate,
    A::Event: Send + Sync,
    E: Send,
    Q: TransactionalEventHandler<A, E>,
    T: Deref<Target = Q> + Send + Sync,
{
    /// Deref call to [`TransactionalEventHandler::handle`].
    async fn handle(&self, event: &StoreEvent<A::Event>, executor: &mut E) -> Result<(), A::Error> {
        self.deref().handle(event, executor).await
    }
}

pub trait ReplayableEventHandler<A>: Sync
where
    Self: EventHandler<A>,
    A: Aggregate,
{
}
