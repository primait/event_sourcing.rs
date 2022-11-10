use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres};
use thiserror::Error;

use esrs::postgres::PgStore;
use esrs::{Aggregate, AggregateManager};

#[derive(Clone)]
pub struct CountingAggregate {
    event_store: PgStore<Self>,
}

impl CountingAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CountingError> {
        let event_store: PgStore<CountingAggregate> = PgStore::new(pool.clone()).setup().await?;

        Ok(Self { event_store })
    }
}

impl Aggregate for CountingAggregate {
    type State = u64;
    type Command = CountingCommand;
    type Event = CountingEvent;
    type Error = CountingError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::Increment => Ok(vec![Self::Event::Incremented]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // This aggregate state just counts the number of applied - equivalent to the number in the event store
        state + 1
    }
}

impl AggregateManager for CountingAggregate {
    type EventStore = PgStore<Self>;

    fn name() -> &'static str {
        "counter"
    }

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

// A simple error enum for event processing errors
#[derive(Debug, Error)]
pub enum CountingError {
    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}

pub enum CountingCommand {
    Increment,
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CountingEvent {
    Incremented,
}
