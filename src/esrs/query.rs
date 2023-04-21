use async_trait::async_trait;
use uuid::Uuid;

use crate::{AggregateManager, StoreEvent};

#[async_trait]
pub trait Query<M: AggregateManager>: Send + Sync {
    async fn handle(&self, event: &StoreEvent<M::Event>);

    async fn delete(&self, _aggregate_id: Uuid) {}

    /// The name of the projector. By default, this is the type name of the projector,
    /// but it can be overridden to provide a custom name. This name is used as
    /// part of tracing spans, to identify the projector being run.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}
//
// #[async_trait]
// impl<M, Q, T> Query<AM> for T
// where
//     AM: AggregateManager,
//     M::Event: Send + Sync,
//     Q: Query<AM>,
//     T: Deref<Target = Q> + Send + Sync,
// {
//     async fn handle(&self, event: StoreEvent<M::Event>) {
//         self.deref().handle(event).await;
//     }
// }

#[async_trait]
pub trait TransactionalQuery<AM: AggregateManager, E> {
    async fn handle(&self, event: &StoreEvent<AM::Event>, executor: &mut E) -> Result<(), AM::Error>;

    async fn delete(&self, aggregate_id: Uuid, executor: &mut E) -> Result<(), AM::Error>;

    /// The name of the projector. By default, this is the type name of the projector,
    /// but it can be overridden to provide a custom name. This name is used as
    /// part of tracing spans, to identify the projector being run.
    fn name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

pub trait QueryError: std::error::Error {}
