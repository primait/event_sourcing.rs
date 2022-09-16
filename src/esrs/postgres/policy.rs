use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use crate::aggregate;
use crate::store::StoreEvent;

#[async_trait]
pub trait Policy<Aggregate>
where
    Aggregate: aggregate::Aggregate,
{
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(
        &self,
        event: &StoreEvent<Aggregate::Event>,
        pool: &Pool<Postgres>,
    ) -> Result<(), Aggregate::Error>;
}
