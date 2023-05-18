use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState};

use crate::common::{new_pool, AggregateA, AggregateB, CommandA, CommandB};
use crate::event_handler::SharedEventHandler;
use crate::view::SharedView;

#[path = "../common/lib.rs"]
mod common;
mod event_handler;
mod view;

const TABLE_NAME: &str = "shared_view";

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let query: String = format!(
        "CREATE TABLE IF NOT EXISTS {} \
        (shared_id uuid PRIMARY KEY NOT NULL, aggregate_id_a uuid, aggregate_id_b uuid, sum INTEGER)",
        TABLE_NAME
    );

    let _ = sqlx::query(query.as_str()).execute(&pool).await.unwrap();

    let shared_event_handler: SharedEventHandler = SharedEventHandler { pool: pool.clone() };

    let store_a: PgStore<AggregateA> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(shared_event_handler.clone())
        .try_build()
        .await
        .unwrap();

    let store_b: PgStore<AggregateB> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(shared_event_handler)
        .try_build()
        .await
        .unwrap();

    let shared_id: Uuid = Uuid::new_v4();

    let state_a: AggregateState<i32> = AggregateState::new();
    let aggregate_id_a: Uuid = *state_a.id();
    AggregateManager::new(store_a)
        .handle_command(state_a, CommandA { v: 5, shared_id })
        .await
        .unwrap();

    let state_b: AggregateState<i32> = AggregateState::new();
    let aggregate_id_b: Uuid = *state_b.id();
    AggregateManager::new(store_b)
        .handle_command(state_b, CommandB { v: 7, shared_id })
        .await
        .unwrap();

    let shared_view = SharedView::by_id(shared_id, &pool).await.unwrap().unwrap();

    assert_eq!(shared_view.shared_id, shared_id);
    assert_eq!(shared_view.aggregate_id_a, Some(aggregate_id_a));
    assert_eq!(shared_view.aggregate_id_b, Some(aggregate_id_b));
    assert_eq!(shared_view.sum, 12); // 5 + 7
}
