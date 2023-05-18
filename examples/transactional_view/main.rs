use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState, EventStore};

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicView, BasicViewRow};
use crate::transactional_event_handler::BasicTransactionalEventHandler;

#[path = "../common/lib.rs"]
mod common;
mod transactional_event_handler;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let view: BasicView = BasicView::new("transactional_view", &pool).await;

    let store: PgStore<BasicAggregate> = PgStoreBuilder::new(pool.clone())
        .add_transactional_event_handler(BasicTransactionalEventHandler { view: view.clone() })
        .try_build()
        .await
        .unwrap();

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();
    let content: &str = "error";

    let command = BasicCommand {
        content: content.to_string(),
    };

    let manager: AggregateManager<BasicAggregate> = AggregateManager::new(store.clone());

    let result = manager.handle_command(state, command).await;

    assert!(result.is_err());
    // No events have been stored. Transactional event handler rollbacked the value
    assert!(store.by_aggregate_id(id).await.unwrap().is_empty());

    let content: &str = "content";
    let command = BasicCommand {
        content: content.to_string(),
    };

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();

    manager.handle_command(state, command).await.unwrap();

    let view: BasicViewRow = view.by_id(id, &pool).await.unwrap().unwrap();

    assert_eq!(view.content, content);
}
