use async_trait::async_trait;
use sqlx::PgConnection;

use esrs::{StoreEvent, TransactionalEventHandler};

use crate::common::{AggregateA, AggregateB, CommonError, EventA, EventB, SharedView, UpsertSharedView};

#[derive(Clone)]
pub struct SharedTransactionalEventHandler {
    pub view: SharedView,
}

#[async_trait]
impl TransactionalEventHandler<AggregateA, PgConnection> for SharedTransactionalEventHandler {
    async fn handle(&self, event: &StoreEvent<EventA>, executor: &mut PgConnection) -> Result<(), CommonError> {
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
impl TransactionalEventHandler<AggregateB, PgConnection> for SharedTransactionalEventHandler {
    async fn handle(&self, event: &StoreEvent<EventB>, executor: &mut PgConnection) -> Result<(), CommonError> {
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
