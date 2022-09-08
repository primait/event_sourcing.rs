use async_trait::async_trait;
use serde::de::DeserializeOwned;
use sqlx::Pool;

use crate::esrs::store::StoreEvent;

#[async_trait]
pub trait Policy<Database, Event, Error>
where
    Database: sqlx::Database,
    Event: DeserializeOwned,
{
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(&self, event: &StoreEvent<Event>, pool: &Pool<Database>) -> Result<(), Error>;
}
