use crate::{Aggregate, StoreEvent};
use async_trait::async_trait;

/// The `EventBus` trait is responsible of the publishing an event on a given bus implementation.
#[async_trait]
pub trait EventBus<A>: Sync
where
    A: Aggregate,
{
    /// Publish an `Aggregate` event on an `EventBus` defined by the user.
    ///
    /// All the errors should be handled from within the `EventBus` and shouldn't panic.
    async fn publish(&self, store_event: &StoreEvent<A::Event>);
}
