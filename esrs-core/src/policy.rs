use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::store::StoreEvent;

#[async_trait]
pub trait Policy<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(&self, event: &StoreEvent<Event>) -> Result<(), Error>;
}
