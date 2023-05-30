use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use esrs::{EventHandler, ReplayableEventHandler, StoreEvent};

/// This is just an example. The need of v1 and v2 is due to having both the version of this event
/// handler compiled in the code. In user codebase there will be only one `BasicEventHandler`
/// got modified.
use crate::common::{BasicAggregate, BasicEvent, BasicView};

#[derive(Clone)]
pub struct BasicEventHandlerV1 {
    pub pool: Pool<Postgres>,
    pub view: BasicView,
}

#[derive(Clone)]
pub struct BasicEventHandlerV2 {
    pub pool: Pool<Postgres>,
    pub view: BasicView,
}

#[derive(Clone)]
pub struct AnotherEventHandler;

#[async_trait]
impl EventHandler<BasicAggregate> for BasicEventHandlerV1 {
    async fn handle(&self, event: &StoreEvent<BasicEvent>) {
        if let Err(e) = self
            .view
            .upsert(event.aggregate_id, format!("{}.v1", &event.payload.content), &self.pool)
            .await
        {
            eprintln!("Error while upserting view: {:?}", e);
        }
    }
}

impl ReplayableEventHandler<BasicAggregate> for BasicEventHandlerV1 {}

#[async_trait]
impl EventHandler<BasicAggregate> for BasicEventHandlerV2 {
    async fn handle(&self, event: &StoreEvent<BasicEvent>) {
        if let Err(e) = self
            .view
            .upsert(event.aggregate_id, format!("{}.v2", &event.payload.content), &self.pool)
            .await
        {
            eprintln!("Error while upserting view: {:?}", e);
        }
    }
}

impl ReplayableEventHandler<BasicAggregate> for BasicEventHandlerV2 {}

#[async_trait]
impl EventHandler<BasicAggregate> for AnotherEventHandler {
    async fn handle(&self, _event: &StoreEvent<BasicEvent>) {}
}
