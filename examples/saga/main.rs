use std::sync::Arc;

use futures::lock::Mutex;
use sqlx::{Pool, Postgres};

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, Boxer};

use crate::aggregate::SagaAggregate;
use crate::common::{new_pool, AggregateA, AggregateB};
use crate::event_handler::SagaEventHandler;

mod aggregate;
#[path = "../common/lib.rs"]
mod common;
mod event_handler;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let side_effect_mutex: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));

    let store: PgStore<SagaAggregate> = PgStoreBuilder::new(pool.clone())
        .try_build()
        .await
        .expect("Failed to build PgStore for AggregateA");

    let saga_event_handler: SagaEventHandler = SagaEventHandler {
        store: store.clone(),
        side_effect_mutex,
    };

    store.add_event_handler(saga_event_handler);
}
