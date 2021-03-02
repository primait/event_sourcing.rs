use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::async_impl::store::StoreEvent;

#[async_trait]
pub trait Projector<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    async fn project(&self, event: &StoreEvent<Event>) -> Result<(), Error>;
}
