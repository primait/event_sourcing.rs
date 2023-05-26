use sqlx::PgConnection;
use uuid::Uuid;

use esrs::postgres::PgStoreError;
use esrs::{StoreEvent, TransactionalEventHandler};

use crate::aggregate::{TestAggregate, TestEvent};

#[derive(Clone)]
pub struct TestTransactionalEventHandler;

#[async_trait::async_trait]
impl TransactionalEventHandler<TestAggregate, PgStoreError, PgConnection> for TestTransactionalEventHandler {
    async fn handle(&self, event: &StoreEvent<TestEvent>, connection: &mut PgConnection) -> Result<(), PgStoreError> {
        let query = "INSERT INTO test_projection (id, total) \
        VALUES ($1, $2)\
        ON CONFLICT (id)\
        DO UPDATE SET total = test_projection.total + 1;";

        Ok(sqlx::query(query)
            .bind(event.aggregate_id)
            .bind(event.payload.add)
            .execute(connection)
            .await
            .map(|_| ())?)
    }

    async fn delete(&self, aggregate_id: Uuid, connection: &mut PgConnection) -> Result<(), PgStoreError> {
        Ok(sqlx::query("DELETE FROM test_projection WHERE id = $1")
            .bind(aggregate_id)
            .execute(connection)
            .await
            .map(|_| ())?)
    }
}
