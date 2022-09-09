use async_trait::async_trait;
use serde::de::DeserializeOwned;
use sqlx::{Pool, Postgres};

use crate::esrs::store::StoreEvent;

#[async_trait]
pub trait Policy<Event, Error>
where
    Event: DeserializeOwned,
{
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(&self, event: &StoreEvent<Event>, pool: &Pool<Postgres>) -> Result<(), Error>;
}
