use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState};

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicEventHandler, BasicView};

#[path = "../common/lib.rs"]
mod common;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let view: BasicView = BasicView::new("eventual_persistent_view", &pool).await;

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
    let content: &str = "content";

    let command = BasicCommand {
        content: content.to_string(),
    };

    AggregateManager::new(store)
        .handle_command(state, command)
        .await
        .expect("Failed to handle command for basic aggregate");

    let row = view
        .by_id(id, &pool)
        .await
        .expect("Failed to get basic view")
        .expect("Basic view entry not found");

    assert_eq!(row.content, content);
}
