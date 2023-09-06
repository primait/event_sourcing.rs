use sqlx::{PgConnection, Pool, Postgres};

use serde::{Deserialize, Serialize};

use esrs::handler::{EventHandler, TransactionalEventHandler};
use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder, PgStoreError};
use esrs::store::StoreEvent;
use esrs::Aggregate;

use esrs::bus::EventBus;
use thiserror::Error;

use crate::common::new_pool;

#[path = "../common/lib.rs"]
mod common;

//////////////////////////////
// Application entry point

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let store: PgStore<Book> = PgStoreBuilder::new(pool)
        .add_event_handler(BookEventHandler)
        .add_transactional_event_handler(BookTransactionalEventHandler)
        .add_event_bus(BookEventBus)
        .try_build()
        .await
        .expect("Failed to create PgStore");

    let manager: AggregateManager<_> = AggregateManager::new(store);
    let _ = manager
        .handle_command(Default::default(), BookCommand::Buy { num_of_copies: 1 })
        .await;
}

//////////////////////////////
// Aggregate

pub struct Book;

impl Aggregate for Book {
    const NAME: &'static str = "book";
    type State = BookState;
    type Command = BookCommand;
    type Event = BookEvent;
    type Error = BookError;

    fn handle_command(state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            BookCommand::Buy { num_of_copies } if state.leftover < num_of_copies => Err(BookError::NotEnoughCopies),
            BookCommand::Buy { num_of_copies } => Ok(vec![BookEvent::Bought { num_of_copies }]),
            BookCommand::Return { num_of_copies } => Ok(vec![BookEvent::Returned { num_of_copies }]),
        }
    }

    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State {
        match payload {
            BookEvent::Bought { num_of_copies } => BookState {
                leftover: state.leftover - num_of_copies,
            },
            BookEvent::Returned { num_of_copies } => BookState {
                leftover: state.leftover + num_of_copies,
            },
        }
    }
}

pub struct BookState {
    pub leftover: i32,
}

impl Default for BookState {
    fn default() -> Self {
        Self { leftover: 10 }
    }
}

pub enum BookCommand {
    Buy { num_of_copies: i32 },
    Return { num_of_copies: i32 },
}

#[derive(Serialize, Deserialize, Clone)]
#[cfg_attr(feature = "upcasting", derive(esrs::Event))]
pub enum BookEvent {
    Bought { num_of_copies: i32 },
    Returned { num_of_copies: i32 },
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for BookEvent {
    fn upcast(value: serde_json::Value, _version: Option<i32>) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

#[derive(Debug, Error)]
pub enum BookError {
    #[error("Not enough copies")]
    NotEnoughCopies,
}

//////////////////////////////
// Event handler

pub struct BookEventHandler;

#[async_trait::async_trait]
impl EventHandler<Book> for BookEventHandler {
    async fn handle(&self, _event: &StoreEvent<BookEvent>) {
        // Implementation here
    }
}

//////////////////////////////
// Transactional event handler

pub struct BookTransactionalEventHandler;

#[async_trait::async_trait]
impl TransactionalEventHandler<Book, PgStoreError, PgConnection> for BookTransactionalEventHandler {
    async fn handle(
        &self,
        _event: &StoreEvent<BookEvent>,
        _transaction: &mut PgConnection,
    ) -> Result<(), PgStoreError> {
        // Implementation here
        Ok(())
    }
}

//////////////////////////////
// Event bus

pub struct BookEventBus;

#[async_trait::async_trait]
impl EventBus<Book> for BookEventBus {
    async fn publish(&self, _store_event: &StoreEvent<BookEvent>) {
        // Implementation here
    }
}
