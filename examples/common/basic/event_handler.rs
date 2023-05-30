use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::{EventHandler, StoreEvent};

use crate::common::{BasicAggregate, BasicEvent, BasicView};

#[derive(Clone)]
pub struct BasicEventHandler {
    pub pool: Pool<Postgres>,
    pub view: BasicView,
}

#[async_trait]
impl EventHandler<BasicAggregate> for BasicEventHandler {
    async fn handle(&self, event: &StoreEvent<BasicEvent>) {
        if let Err(e) = self
            .view
            .upsert(event.aggregate_id, event.payload.content.to_string(), &self.pool)
            .await
        {
            eprintln!("Error while upserting view: {:?}", e);
        }
    }

    async fn delete(&self, aggregate_id: Uuid) {
        if let Err(e) = self.view.delete(aggregate_id, &self.pool).await {
            eprintln!("Error while deleting view: {:?}", e);
        }
    }
}
