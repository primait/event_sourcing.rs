use serde::de::DeserializeOwned;
use serde::Serialize;
use std::future::Future;
use std::pin::Pin;
use uuid::Uuid;

use crate::esrs::store::StoreEvent;

/// Projector trait that takes an executor in order to create a read model
pub trait Projector<Event: Serialize + DeserializeOwned + Send + Sync, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    fn project<'a>(
        &'a self,
        event: &'a StoreEvent<Event>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>;

    /// This function delete projections by aggregate_id. By default do nothing. Implement this in
    /// order to achieve the deletion task.
    fn delete<'a>(&'a self, _aggregate_id: Uuid) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>> {
        Box::pin(async move { Ok(()) })
    }
}
