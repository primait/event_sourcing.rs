use async_trait::async_trait;
use sqlx::PgConnection;

use esrs::postgres::PgStoreError;
use esrs::{StoreEvent, TransactionalEventHandler};

use crate::common::{BasicAggregate, BasicEvent, BasicView};

pub struct BasicTransactionalEventHandler {
    pub view: BasicView,
}

#[async_trait]
impl TransactionalEventHandler<BasicAggregate, PgStoreError, PgConnection> for BasicTransactionalEventHandler {
    async fn handle(&self, event: &StoreEvent<BasicEvent>, transaction: &mut PgConnection) -> Result<(), PgStoreError> {
        // This is to show that event is rollbacked
        if event.payload.content.eq("error") {
            return Err(PgStoreError::Custom(Box::new(BasicEventHandlerError::Custom(
                "Event contains `error` string".to_string(),
            ))));
        }

        let result = self
            .view
            .upsert(event.aggregate_id, event.payload.content.to_string(), transaction)
            .await;

        if let Err(e) = result {
            eprintln!("Error while upserting view: {:?}", e);
            Err(e.into())
        } else {
            Ok(())
        }
    }
}

#[derive(thiserror::Error, Debug)]
pub enum BasicEventHandlerError {
    #[error("{0}")]
    Custom(String),
}
