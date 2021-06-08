use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::Sqlite;

use crate::esrs::pool::Pool;
use crate::esrs::store::StoreEvent;

#[async_trait]
pub trait SqlitePolicy<Event: Serialize + DeserializeOwned + Clone + Send + Sync, Error> {
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(&self, event: &StoreEvent<Event>, pool: &Pool<Sqlite>) -> Result<(), Error>;
}
