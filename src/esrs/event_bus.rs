use async_trait::async_trait;

use crate::{Aggregate, StoreEvent};

#[cfg(feature = "kafka")]
mod kafka;

#[async_trait]
pub trait EventBus<A>: Sync
where
    A: Aggregate,
{
    /// Publish an Aggregate event on an Event bus defined by the user.
    async fn publish(&self, store_event: &StoreEvent<A::Event>);
}
