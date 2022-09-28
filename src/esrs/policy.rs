use async_trait::async_trait;

use crate::{AggregateManager, StoreEvent};

/// This trait is used to implement a `Policy`. A policy is intended to be an entity where to put
/// non-transactional side effects.
#[async_trait]
pub trait Policy<Manager>
where
    Manager: AggregateManager,
{
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(&self, event: &StoreEvent<Manager::Event>) -> Result<(), Manager::Error>;
}
