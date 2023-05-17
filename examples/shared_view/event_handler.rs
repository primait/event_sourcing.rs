use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use esrs::{EventHandler, StoreEvent};

use crate::common::{AggregateA, AggregateB, EventA, EventB};
use crate::view::{SharedView, Upsert};

#[derive(Clone)]
pub struct SharedEventHandler {
    pub pool: Pool<Postgres>,
}

#[async_trait]
impl EventHandler<AggregateA> for SharedEventHandler {
    async fn handle(&self, event: &StoreEvent<EventA>) {
        let result = SharedView::upsert(
            Upsert::A {
                shared_id: event.payload.shared_id,
                aggregate_id: event.aggregate_id,
                value: event.payload.v,
            },
            &self.pool,
        )
        .await;

        if let Err(e) = result {
            println!("Error inserting A to shared view: {:?}", e)
        }
    }
}

#[async_trait]
impl EventHandler<AggregateB> for SharedEventHandler {
    async fn handle(&self, event: &StoreEvent<EventB>) {
        let result = SharedView::upsert(
            Upsert::B {
                shared_id: event.payload.shared_id,
                aggregate_id: event.aggregate_id,
                value: event.payload.v,
            },
            &self.pool,
        )
        .await;

        if let Err(e) = result {
            println!("Error inserting B to shared view: {:?}", e)
        }
    }
}
