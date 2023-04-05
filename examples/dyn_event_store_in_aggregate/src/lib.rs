use sqlx::{Pool, Postgres};

use esrs::postgres::PgStore;
use esrs::{Aggregate, AggregateManager, EventStore};

pub struct CounterAggregate {
    pub event_store: Box<dyn EventStore<Manager = Self> + Send + Sync>,
}

impl CounterAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        let event_store: PgStore<Self> = PgStore::new(pool.clone()).setup().await?;

        Ok(Self {
            event_store: Box::new(event_store),
        })
    }
}

impl Aggregate for CounterAggregate {
    type State = ();
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::Increment => Ok(vec![Self::Event::Incremented]),
            Self::Command::Decrement => Ok(vec![Self::Event::Decremented]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection
        state
    }
}

impl AggregateManager for CounterAggregate {
    type EventStore = Box<dyn EventStore<Manager = Self> + Send + Sync>;

    fn name() -> &'static str {
        "counter"
    }

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

// A simple error enum for event processing errors
#[derive(Debug, thiserror::Error)]
pub enum CounterError {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}

// The events to be projected
#[derive(serde::Serialize, serde::Deserialize, Debug)]
pub enum CounterEvent {
    Incremented,
    Decremented,
}

// The commands received by the application, which will produce the events
pub enum CounterCommand {
    Increment,
    Decrement,
}
