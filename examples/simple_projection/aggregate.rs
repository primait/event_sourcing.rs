use sqlx::{Pool, Postgres};

use esrs::postgres::PgStore;
use esrs::{Aggregate, AggregateManager};

use crate::projector::CounterProjector;
use crate::structs::{CounterCommand, CounterError, CounterEvent};

pub struct CounterAggregate {
    pub event_store: PgStore<Self>,
}

impl CounterAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        let event_store: PgStore<Self> = PgStore::new(pool.clone())
            .set_projectors(vec![Box::new(CounterProjector)])
            .setup()
            .await?;

        Ok(Self { event_store })
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
    type EventStore = PgStore<Self>;

    fn name() -> &'static str {
        "counter"
    }

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}
