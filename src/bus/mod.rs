use async_trait::async_trait;

use crate::store::StoreEvent;
use crate::Aggregate;

#[cfg(feature = "kafka")]
pub mod kafka;
#[cfg(feature = "rabbit")]
pub mod rabbit;

/// The responsibility of the [`EventBus`] trait is to publish an event on a specific bus implementation.
#[async_trait]
pub trait EventBus<A>: Sync
where
    A: Aggregate,
{
    /// Publish an [`Aggregate`] event on an [`EventBus`] defined by the user.
    ///
    /// All the errors should be handled from within the [`EventBus`] and shouldn't panic.
    async fn publish(&self, store_event: &StoreEvent<A::Event>);
}
