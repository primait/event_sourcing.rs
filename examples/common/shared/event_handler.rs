use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use esrs::handler::{EventHandler, ReplayableEventHandler};
use esrs::store::StoreEvent;

use crate::common::a::{AggregateA, EventA};
use crate::common::b::{AggregateB, EventB};
use crate::common::shared::view::{SharedView, UpsertSharedView};

#[derive(Clone)]
pub struct SharedEventHandler {
    pub pool: Pool<Postgres>,
    pub view: SharedView,
}

#[async_trait]
impl EventHandler<AggregateA> for SharedEventHandler {
    async fn handle(&self, event: &StoreEvent<EventA>) {
        let result = self
            .view
            .upsert(
                UpsertSharedView::A {
                    shared_id: event.payload.shared_id,
                    aggregate_id: event.aggregate_id,
                    value: event.payload.v,
                },
                &self.pool,
            )
            .await;

        if let Err(e) = result {
            eprintln!("Error inserting A to shared view: {:?}", e)
        }
    }
}

#[async_trait]
impl EventHandler<AggregateB> for SharedEventHandler {
    async fn handle(&self, event: &StoreEvent<EventB>) {
        let result = self
            .view
            .upsert(
                UpsertSharedView::B {
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

impl ReplayableEventHandler<AggregateA> for SharedEventHandler {}

impl ReplayableEventHandler<AggregateB> for SharedEventHandler {}
