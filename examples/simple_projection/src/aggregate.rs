use async_trait::async_trait;
use esrs::projector::SqliteProjector;
use sqlx::{Pool, Sqlite};

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState, Identifier};
use esrs::store::{EventStore, SqliteStore};

use crate::projector::CounterProjector;
use crate::structs::{CounterCommand, CounterError, CounterEvent};

const COUNTERS: &str = "counter";

// A store of events
pub type CounterStore = SqliteStore<CounterEvent, CounterError>;

pub struct CounterAggregate {
    event_store: CounterStore,
}

impl CounterAggregate {
    pub async fn new(pool: &Pool<Sqlite>) -> Result<Self, CounterError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    async fn new_store(pool: &Pool<Sqlite>) -> Result<CounterStore, CounterError> {
        let projectors: Vec<Box<dyn SqliteProjector<CounterEvent, CounterError> + Send + Sync>> =
            vec![Box::new(CounterProjector)];

        SqliteStore::new::<Self>(pool, projectors, vec![]).await
    }
}

impl Identifier for CounterAggregate {
    fn name() -> &'static str {
        COUNTERS
    }
}

#[async_trait]
impl Aggregate for CounterAggregate {
    type State = ();
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn handle_command(
        _state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::Increment => Ok(vec![Self::Event::Incremented]),
            Self::Command::Decrement => Ok(vec![Self::Event::Decremented]),
        }
    }

    fn apply_event(state: Self::State, _: &Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection
        state
    }
}

impl AggregateManager for CounterAggregate {
    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }
}
