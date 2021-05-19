use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Postgres};

use crate::esrs::store::StoreEvent;

#[async_trait]
pub trait PgPolicy<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(&self, event: &StoreEvent<Event>, pool: &Pool<Postgres>) -> Result<(), Error>;
}
