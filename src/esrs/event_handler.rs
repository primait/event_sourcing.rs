use async_trait::async_trait;
use std::ops::Deref;
use uuid::Uuid;

use crate::{Aggregate, StoreEvent};

#[async_trait]
pub trait EventHandler<A>: Sync
where
    A: Aggregate,
{
    // TODO: doc
    async fn handle(&self, event: &StoreEvent<A::Event>);

    // TODO: doc
    async fn delete(&self, _aggregate_id: Uuid) {}

    /// The name of the event handler. By default, this is the type name of the event handler,
    /// but it can be overridden to provide a custom name. This name is used as
    /// part of tracing spans, to identify the event handler being run.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

// FIXME: uncomment
#[async_trait]
impl<A, Q, T> EventHandler<A> for T
where
    A: Aggregate,
    A::Event: Send + Sync,
    Q: EventHandler<A>,
    T: Deref<Target = Q> + Send + Sync,
{
    async fn handle(&self, event: &StoreEvent<A::Event>) {
        self.deref().handle(event).await;
    }
}

#[async_trait]
pub trait TransactionalEventHandler<A, E>: Sync
where
    A: Aggregate,
{
    // TODO: doc
    async fn handle(&self, event: &StoreEvent<A::Event>, executor: &mut E) -> Result<(), A::Error>;

    // TODO: doc
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
