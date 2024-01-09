//! In this example, we provide a demonstration of building a view using an [`EventHandler`]. The
//! example illustrates the process of handling events and processing them to construct a
//! comprehensive view of the underlying data.

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder, PgStoreError};
use esrs::AggregateState;

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicError, BasicEventHandler, BasicView};

#[path = "../common/lib.rs"]
mod common;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error(transparent)]
    Aggregate(#[from] BasicError),
    #[error(transparent)]
    Store(#[from] PgStoreError),
}

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
        .unwrap();

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();
    let content: &str = "content";

    let command = BasicCommand {
        content: content.to_string(),
    };

    AggregateManager::new(store)
        .handle_command::<Error>(state, command)
        .await
        .unwrap();

    let row = view.by_id(id, &pool).await.unwrap().unwrap();

    assert_eq!(row.content, content);
}
