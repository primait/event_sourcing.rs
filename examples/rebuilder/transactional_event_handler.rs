use async_trait::async_trait;
use sqlx::PgConnection;

use esrs::handler::TransactionalEventHandler;
use esrs::store::postgres::PgStoreError;
use esrs::store::StoreEvent;

use crate::common::basic::view::BasicView;
use crate::common::basic::{BasicAggregate, BasicEvent};

/// The `BasicTransactionalEventHandlerV1` and `BasicTransactionalEventHandlerV1` exists in this
/// example just for the sake of showing how a single transactional event handler, in this case called
/// `BasicTransactionalEventHandler`, changed overtime. In this example is needed to have both the
/// v1 and v2 version in order to be able to run both the code versions.

pub struct BasicTransactionalEventHandlerV1 {
    pub view: BasicView,
}

pub struct BasicTransactionalEventHandlerV2 {
    pub view: BasicView,
}

#[async_trait]
impl TransactionalEventHandler<BasicAggregate, PgStoreError, PgConnection> for BasicTransactionalEventHandlerV1 {
    async fn handle(&self, event: &StoreEvent<BasicEvent>, transaction: &mut PgConnection) -> Result<(), PgStoreError> {
        Ok(self
            .view
            .upsert(
                event.aggregate_id,
                format!("{}.v1", &event.payload.content),
                transaction,
            )
            .await?)
    }
}

#[async_trait]
impl TransactionalEventHandler<BasicAggregate, PgStoreError, PgConnection> for BasicTransactionalEventHandlerV2 {
    async fn handle(&self, event: &StoreEvent<BasicEvent>, transaction: &mut PgConnection) -> Result<(), PgStoreError> {
        Ok(self
            .view
            .upsert(
                event.aggregate_id,
                format!("{}.v2", &event.payload.content),
                transaction,
            )
            .await?)
    }
}
