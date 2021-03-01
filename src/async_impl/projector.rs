use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(test)]
use mockall::automock;

use crate::async_impl::store::StoreEvent;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait Projector<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    /// The content of the result is an Option because it is not ensured that the projection
    /// of an event generates insertion in the specific read model.
    async fn project(&self, event: &StoreEvent<Event>) -> Result<(), Error>;
}
