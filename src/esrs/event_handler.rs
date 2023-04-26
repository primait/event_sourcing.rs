use async_trait::async_trait;
use uuid::Uuid;

use crate::{AggregateManager, StoreEvent};

#[async_trait]
pub trait EventHandler<M: AggregateManager>: Send + Sync {
    // TODO: doc
    async fn handle(&self, event: &StoreEvent<M::Event>);

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
// #[async_trait]
// impl<M, Q, T> EventHandler<AM> for T
// where
//     AM: AggregateManager,
//     M::Event: Send + Sync,
//     Q: EventHandler<AM>,
//     T: Deref<Target = Q> + Send + Sync,
// {
//     async fn handle(&self, event: StoreEvent<M::Event>) {
//         self.deref().handle(event).await;
//     }
// }

#[async_trait]
pub trait TransactionalEventHandler<AM, E>: Sync
where
    AM: AggregateManager,
{
    // TODO: doc
    async fn handle(&self, event: &StoreEvent<AM::Event>, executor: &mut E) -> Result<(), AM::Error>;

    // TODO: doc
    async fn delete(&self, _aggregate_id: Uuid, _executor: &mut E) -> Result<(), AM::Error> {
        Ok(())
    }

    /// The name of the event handler. By default, this is the type name of the event handler,
    /// but it can be overridden to provide a custom name. This name is used as
    /// part of tracing spans, to identify the event handler being run.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait ReplayableEventHandler<M: AggregateManager>: EventHandler<M> + Send + Sync {}

// TODO: doc
pub trait EventHandlerError: std::error::Error {}
