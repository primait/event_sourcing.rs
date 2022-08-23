use async_trait::async_trait;
use esrs::projector::SqliteProjector;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use esrs::aggregate::{AggregateManager, AggregateState, Identifier};
use esrs::store::{EventStore, SqliteStore, StoreEvent};

use crate::projector::CounterProjector;
use crate::structs::{CounterCommand, CounterError, CounterEvent};

const COUNTERS: &str = "counters";

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

    pub async fn new_store(pool: &Pool<Sqlite>) -> Result<CounterStore, CounterError> {
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
impl AggregateManager for CounterAggregate {
    type State = ();
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }

    fn apply_event(_id: &Uuid, state: Self::State, _: &StoreEvent<Self::Event>) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }

    fn validate_command(_: &AggregateState<Self::State>, _: &Self::Command) -> Result<(), Self::Error> {
        Ok(()) // No validation done on commands received in this aggregate
    }

    async fn do_handle_command(
        &self,
        aggregate_state: AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        match cmd {
            Self::Command::Increment => self.persist(aggregate_state, vec![Self::Event::Incremented]).await,
            Self::Command::Decrement => self.persist(aggregate_state, vec![Self::Event::Decremented]).await,
        }
    }
}
