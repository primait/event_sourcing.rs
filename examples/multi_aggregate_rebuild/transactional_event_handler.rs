use async_trait::async_trait;
use sqlx::PgConnection;

use esrs::handler::TransactionalEventHandler;
use esrs::store::postgres::PgStoreError;
use esrs::store::StoreEvent;

use crate::common::a::{AggregateA, EventA};
use crate::common::b::{AggregateB, EventB};
use crate::common::shared::view::{SharedView, UpsertSharedView};

#[derive(Clone)]
pub struct SharedTransactionalEventHandler {
    pub view: SharedView,
}

#[async_trait]
impl TransactionalEventHandler<AggregateA, PgStoreError, PgConnection> for SharedTransactionalEventHandler {
    async fn handle(&self, event: &StoreEvent<EventA>, executor: &mut PgConnection) -> Result<(), PgStoreError> {
        Ok(self
            .view
            .upsert(
                UpsertSharedView::A {
                    shared_id: event.payload.shared_id,
                    aggregate_id: event.aggregate_id,
                    value: event.payload.v,
                },
                executor,
            )
            .await?)
    }
}

#[async_trait]
impl TransactionalEventHandler<AggregateB, PgStoreError, PgConnection> for SharedTransactionalEventHandler {
    async fn handle(&self, event: &StoreEvent<EventB>, executor: &mut PgConnection) -> Result<(), PgStoreError> {
        Ok(self
            .view
            .upsert(
                UpsertSharedView::B {
                    shared_id: event.payload.shared_id,
                    aggregate_id: event.aggregate_id,
                    value: event.payload.v,
                },
                executor,
            )
            .await?)
    }
}
