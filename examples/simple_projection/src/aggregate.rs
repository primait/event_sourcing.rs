use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState};
use esrs::store::postgres::PgStore;
use esrs::store::postgres::Projector;

use crate::projector::CounterProjector;
use crate::structs::{CounterCommand, CounterError, CounterEvent};

// A store of events
pub type CounterStore = PgStore<CounterEvent, CounterError>;

pub struct CounterAggregate {
    event_store: CounterStore,
}

impl CounterAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    async fn new_store(pool: &Pool<Postgres>) -> Result<CounterStore, CounterError> {
        let projectors: Vec<Box<dyn Projector<CounterEvent, CounterError> + Send + Sync>> =
            vec![Box::new(CounterProjector)];

        PgStore::new::<Self>(pool, projectors, vec![]).setup().await
    }
}

#[async_trait]
impl Aggregate for CounterAggregate {
    type State = ();
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn name() -> &'static str {
        "counter"
    }

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
    type EventStore = CounterStore;

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}
