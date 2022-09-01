use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::esrs::store::StoreEvent;

#[async_trait]
pub trait Policy<Executor, Event: Serialize + DeserializeOwned + Send + Sync, Error> {
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(&self, event: &StoreEvent<Event>, executor: &Executor) -> Result<(), Error>;
}
