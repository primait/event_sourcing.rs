//! In this example, we provide a demonstration of building a view using an [`EventHandler`]. The
//! example illustrates the process of handling events and processing them to construct a
//! comprehensive view of the underlying data.

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::AggregateState;

use crate::common::basic::event_handler::BasicEventHandler;
use crate::common::basic::view::BasicView;
use crate::common::basic::{BasicAggregate, BasicCommand};
use crate::common::util::new_pool;

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
        .unwrap();

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();
    let content: &str = "content";

    let command = BasicCommand {
        content: content.to_string(),
    };

    AggregateManager::new(store)
        .handle_command(state, command)
        .await
        .unwrap()
        .unwrap();

    let row = view.by_id(id, &pool).await.unwrap().unwrap();

    assert_eq!(row.content, content);
}
