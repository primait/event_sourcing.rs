use std::ops::Deref;

use async_trait::async_trait;
use uuid::Uuid;

use crate::{Aggregate, StoreEvent};

/// This trait is used to implement an `EventHandler`. An event handler is intended to be an entity
/// which can create, update and delete a read side and perform side effects.
///
/// The main purpose of an `EventHandler` is to have an eventually persistent processor.
#[async_trait]
pub trait EventHandler<A>: Sync
where
    Self: EventHandlerClone<A>,
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
    Q: EventHandler<A> + EventHandlerClone<A>,
    T: Deref<Target = Q> + Clone + Send + Sync + 'static,
{
    /// Deref call to [`EventHandler::handle`].
    async fn handle(&self, event: &StoreEvent<A::Event>) {
        self.deref().handle(event).await;
    }

    /// Deref call to [`EventHandler::handle`].
    async fn delete(&self, aggregate_id: Uuid) {
        self.deref().delete(aggregate_id).await;
    }

    /// Deref call to [`EventHandler::handle`].
    fn name(&self) -> &'static str {
        self.deref().name()
    }
}

/// This trait is used to implement a `TransactionalEventHandler`. A transactional event handler is
/// intended to be an entity which can create, update and delete a read side. No side effects must be
/// performed inside of this kind on handler.
///
/// An `handle` operation will result in a _deadlock_ if the implementation of this trait is used to
/// apply an event on an [`Aggregate`].
#[async_trait]
pub trait TransactionalEventHandler<A, Error, Executor>: Sync
where
    A: Aggregate,
{
    /// Handle an event in a transactional fashion and perform a read side crate, update or delete.
    /// If an error is returned the transaction will be aborted and the handling of a command by an
    /// aggregate will return an error.
    async fn handle(&self, event: &StoreEvent<A::Event>, executor: &mut Executor) -> Result<(), Error>;

    /// Perform a deletion of a read side projection using the given aggregate_id.
    async fn delete(&self, _aggregate_id: Uuid, _executor: &mut Executor) -> Result<(), Error> {
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
impl<A, Error, Executor, Q, T> TransactionalEventHandler<A, Error, Executor> for T
where
    A: Aggregate,
    A::Event: Send + Sync,
    // Error: Send,
    Executor: Send,
    Q: TransactionalEventHandler<A, Error, Executor>,
    T: Deref<Target = Q> + Send + Sync,
{
    /// Deref call to [`TransactionalEventHandler::handle`].
    async fn handle(&self, event: &StoreEvent<A::Event>, executor: &mut Executor) -> Result<(), Error> {
        self.deref().handle(event, executor).await
    }

    /// Deref call to [`TransactionalEventHandler::delete`].
    async fn delete(&self, aggregate_id: Uuid, executor: &mut Executor) -> Result<(), Error> {
        self.deref().delete(aggregate_id, executor).await
    }

    /// Deref call to [`TransactionalEventHandler::name`].
    fn name(&self) -> &'static str {
        self.deref().name()
    }
}

/// The `ReplayableEventHandler` trait is used to add the `replay` behavior on an `EventHandler`.
///
/// Being replayable means that the operation performed by this EventHandler should be idempotent
/// and should be intended to be "eventually consistent".
/// In other words it means that they should not perform external API calls, generate random numbers
/// or do anything that relies on external state and might change the outcome of this function.
///
/// The most common use case for this is when rebuilding read models: `EventHandler`s that write on
/// the database should be marked as replayable.
///
/// Another use case could be if there's the need to implement a retry logic for this event handler.
pub trait ReplayableEventHandler<A>: Sync
where
    Self: EventHandler<A>,
    A: Aggregate,
{
}

pub trait EventHandlerClone<A> {
    fn clone_box(&self) -> Box<dyn EventHandler<A> + Send>;
}

impl<T, A> EventHandlerClone<A> for T
where
    A: Aggregate,
    T: 'static + EventHandler<A> + Clone + Send,
{
    fn clone_box(&self) -> Box<dyn EventHandler<A> + Send> {
        Box::new(self.clone())
    }
}

// We can now implement Clone manually by forwarding to clone_box.
impl<A> Clone for Box<dyn EventHandler<A> + Send> {
    fn clone(&self) -> Box<dyn EventHandler<A> + Send> {
        self.clone_box()
    }
}
