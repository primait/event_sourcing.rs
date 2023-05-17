use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState, Boxer, EventStore};

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicView};
use crate::transactional_event_handler::BasicTransactionalEventHandler;

#[path = "../common/lib.rs"]
mod common;
mod transactional_event_handler;

const TABLE_NAME: &str = "transactional_view";

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let query: String = format!(
        "CREATE TABLE IF NOT EXISTS {} (id uuid PRIMARY KEY NOT NULL, content VARCHAR)",
        TABLE_NAME
    );

    let _ = sqlx::query(query.as_str())
        .execute(&pool)
        .await
        .expect("Failed to create shared_view table");

    let store: PgStore<BasicAggregate> = PgStoreBuilder::new(pool.clone())
        .add_transactional_event_handler(BasicTransactionalEventHandler.boxed())
        .try_build()
        .await
        .expect("Failed to build PgStore for basic aggregate");

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();
    let content: &str = "error";

    let command = BasicCommand {
        content: content.to_string(),
    };

    let manager: AggregateManager<BasicAggregate> = AggregateManager::new(store.clone().boxed());

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

    manager
        .handle_command(state, command)
        .await
        .expect("Failed to handle command");

    let view: BasicView = BasicView::by_id(id, &pool)
        .await
        .expect("Failed to get basic view")
        .expect("Basic view entry not found");

    assert_eq!(view.content, content);

    dbg!(view);
}
