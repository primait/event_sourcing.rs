//! The purpose of this example is to demonstrate the process of deleting an aggregate and its
//! projections using the [`PgStore`].

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::store::{EventStore, StoreEvent};
use esrs::AggregateState;

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
        .unwrap();

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();
    let content: &str = "content to delete";

    let command = BasicCommand {
        content: content.to_string(),
    };

    let manager = AggregateManager::new(store.clone());

    manager.handle_command(state, command).await.unwrap();

    let row = view.by_id(id, &pool).await.unwrap().unwrap();

    assert_eq!(row.content, content);

    manager.delete(id).await.unwrap();

    let events: Vec<StoreEvent<BasicEvent>> = store.by_aggregate_id(id).await.unwrap();

    assert!(events.is_empty());

    let row = view.by_id(id, &pool).await.unwrap();
    assert!(row.is_none());
}
