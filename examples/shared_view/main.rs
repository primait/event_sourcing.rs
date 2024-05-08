//! This example demonstrates how to construct a view that encompasses two distinct [`Aggregates`].

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::AggregateState;

use crate::common::a::{AggregateA, CommandA};
use crate::common::b::{AggregateB, CommandB};
use crate::common::shared::event_handler::SharedEventHandler;
use crate::common::shared::view::SharedView;
use crate::common::util::new_pool;

#[path = "../common/lib.rs"]
mod common;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let shared_view: SharedView = SharedView::new("shared_view", &pool).await;

    let shared_event_handler: SharedEventHandler = SharedEventHandler {
        pool: pool.clone(),
        view: shared_view.clone(),
    };

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
        .unwrap()
        .unwrap();

    let state_b: AggregateState<i32> = AggregateState::new();
    let aggregate_id_b: Uuid = *state_b.id();
    AggregateManager::new(store_b)
        .handle_command(state_b, CommandB { v: 7, shared_id })
        .await
        .unwrap()
        .unwrap();

    let shared_view = shared_view.by_id(shared_id, &pool).await.unwrap().unwrap();

    assert_eq!(shared_view.shared_id, shared_id);
    assert_eq!(shared_view.aggregate_id_a, Some(aggregate_id_a));
    assert_eq!(shared_view.aggregate_id_b, Some(aggregate_id_b));
    assert_eq!(shared_view.sum, 12); // 5 + 7
}
