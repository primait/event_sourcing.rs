use async_trait::async_trait;
use sqlx::PgConnection;

use esrs::{StoreEvent, TransactionalEventHandler};

use crate::common::{BasicAggregate, BasicError, BasicEvent, BasicView};

/// This is just an example. The need of v1 and v2 is due to having both the version of this
/// transactional event handler compiled in the code. In user codebase there will be only one
/// `BasicTransactionalEventHandler` got modified.

pub struct BasicTransactionalEventHandlerV1 {
    pub view: BasicView,
}

pub struct BasicTransactionalEventHandlerV2 {
    pub view: BasicView,
}

#[async_trait]
impl TransactionalEventHandler<BasicAggregate, PgConnection> for BasicTransactionalEventHandlerV1 {
    async fn handle(&self, event: &StoreEvent<BasicEvent>, transaction: &mut PgConnection) -> Result<(), BasicError> {
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
impl TransactionalEventHandler<BasicAggregate, PgConnection> for BasicTransactionalEventHandlerV2 {
    async fn handle(&self, event: &StoreEvent<BasicEvent>, transaction: &mut PgConnection) -> Result<(), BasicError> {
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
