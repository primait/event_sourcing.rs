//! This example showcases the process of constructing a view using a [`TransactionalEventHandler`].
//! The [`TransactionalEventHandler`] ensures strong consistency by executing all operations within
//! a transactional context. It guarantees that if a failure occurs during the execution of the
//! transaction, the event will not be persisted in the store, and an error will be returned to the
//! caller.
//!
//! By utilizing the `TransactionalEventHandler`, you can maintain a strong and reliable event
//! handling mechanism. The example emphasizes the importance of data integrity, as any failure in
//! the `TransactionalEventHandler` prevents the event from being stored, ensuring the consistency
//! of the view and the event store.
//!
//! This demonstration serves as a practical illustration of how the `TransactionalEventHandler`
//! helps enforce strong consistency in view construction. It highlights the reliability of the
//! approach by ensuring that events are either fully processed and persisted or not persisted at
//! all in the event of a failure, providing a clear and consistent state of the data.

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder, PgStoreError};
use esrs::store::EventStore;
use esrs::AggregateState;

use crate::common::basic::view::{BasicView, BasicViewRow};
use crate::common::basic::{BasicAggregate, BasicCommand, BasicError};
use crate::common::util::new_pool;
use crate::transactional_event_handler::BasicTransactionalEventHandler;

#[path = "../common/lib.rs"]
mod common;
mod transactional_event_handler;

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

    let manager: AggregateManager<PgStore<BasicAggregate>> = AggregateManager::new(store.clone());

    let result: Result<(), Error> = manager.handle_command(state, command).await;

    assert!(result.is_err());
    // No events have been stored. Transactional event handler rollbacked the value
    assert!(store.by_aggregate_id(id).await.unwrap().is_empty());

    let content: &str = "content";
    let command = BasicCommand {
        content: content.to_string(),
    };

    let state: AggregateState<()> = AggregateState::new();
    let id: Uuid = *state.id();

    let result: Result<(), Error> = manager.handle_command(state, command).await;
    assert!(result.is_ok());

    let view: BasicViewRow = view.by_id(id, &pool).await.unwrap().unwrap();
    assert_eq!(view.content, content);
}
