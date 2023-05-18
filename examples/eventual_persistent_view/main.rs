use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState};

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicView, BASIC_TABLE_NAME};
use crate::event_handler::BasicEventHandler;

#[path = "../common/lib.rs"]
mod common;
mod event_handler;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let query: String = format!(
        "CREATE TABLE IF NOT EXISTS {} (id uuid PRIMARY KEY NOT NULL, content VARCHAR)",
        BASIC_TABLE_NAME
    );

    let _ = sqlx::query(query.as_str())
        .execute(&pool)
        .await
        .expect("Failed to create shared_view table");

    let store: PgStore<BasicAggregate> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(BasicEventHandler { pool: pool.clone() })
        .try_build()
        .await
        .expect("Failed to build PgStore for basic aggregate");

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();
    let content: &str = "content";

    let command = BasicCommand {
        content: content.to_string(),
    };

    AggregateManager::new(store)
        .handle_command(state, command)
        .await
        .expect("Failed to handle command for basic aggregate");

    let view = BasicView::by_id(id, &pool)
        .await
        .expect("Failed to get basic view")
        .expect("Basic view entry not found");

    assert_eq!(view.content, content);

    dbg!(view);
}
