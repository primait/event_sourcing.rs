use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use esrs::{EventHandler, StoreEvent};

use crate::common::{BasicAggregate, BasicEvent, BasicView};

#[derive(Clone)]
pub struct BasicEventHandler {
    pub pool: Pool<Postgres>,
}

#[async_trait]
impl EventHandler<BasicAggregate> for BasicEventHandler {
    async fn handle(&self, event: &StoreEvent<BasicEvent>) {
        if let Err(e) = BasicView::upsert(event.aggregate_id, event.payload.content.to_string(), &self.pool).await {
            eprintln!("Error while upserting view: {:?}", e);
        }
    }
}
