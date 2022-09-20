use sqlx::{Pool, Postgres};

use esrs::postgres::PgStore;
use esrs::{Aggregate, AggregateManager, AggregateState};

use crate::projector::CounterProjector;
use crate::structs::{CounterCommand, CounterError, CounterEvent};

pub struct CounterAggregate {
    event_store: PgStore<Self>,
}

impl CounterAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        let mut event_store: PgStore<CounterAggregate> = PgStore::new(pool.clone()).setup().await?;
        event_store.add_projector(Box::new(CounterProjector));

        Ok(Self { event_store })
    }
}

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
