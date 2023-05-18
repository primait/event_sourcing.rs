use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState, EventStore, StoreEvent};

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicEvent, BasicEventHandler, BasicView};

#[path = "../common/lib.rs"]
mod common;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let view: BasicView = BasicView::new("full_aggregate_deletion_view", &pool).await;

    let store: PgStore<BasicAggregate> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(BasicEventHandler {
            pool: pool.clone(),
            view: view.clone(),
        })
        .try_build()
        .await
        .expect("Failed to build PgStore for basic aggregate");

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();
    let content: &str = "content to delete";

    let command = BasicCommand {
        content: content.to_string(),
    };

    let manager = AggregateManager::new(store.clone());

    manager
        .handle_command(state, command)
        .await
        .expect("Failed to handle command for basic aggregate");

    let row = view
        .by_id(id, &pool)
        .await
        .expect("Failed to get basic view")
        .expect("Basic view entry not found");

    assert_eq!(row.content, content);

    manager.delete(id).await.expect("Failed to delete aggregate");

    let events: Vec<StoreEvent<BasicEvent>> = store
        .by_aggregate_id(id)
        .await
        .expect("Failed to get events from store");

    assert!(events.is_empty());

    let row = view.by_id(id, &pool).await.expect("Failed to get row from view");
    assert!(row.is_none());
}
