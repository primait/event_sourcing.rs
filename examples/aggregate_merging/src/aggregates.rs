use sqlx::{Pool, Postgres};

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState};
use esrs::store::postgres::PgStore;
use esrs::store::postgres::Projector;

use crate::projectors::CounterProjector;
use crate::structs::{CommandA, CommandB, CounterError, EventA, EventB};

// We use a template here to make instantiating the near-identical
// AggregateA and AggregateB easier.
pub struct AggregateA {
    event_store: PgStore<Self>,
}

impl AggregateA {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    pub async fn new_store(pool: &Pool<Postgres>) -> Result<PgStore<Self>, CounterError> {
        let projectors: Vec<Box<dyn Projector<Self> + Send + Sync>> = vec![Box::new(CounterProjector)];

        PgStore::new(pool, projectors, vec![]).setup().await
    }
}

impl Aggregate for AggregateA {
    type State = ();
    type Command = CommandA;
    type Event = EventA;
    type Error = CounterError;

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "a"
    }

    fn handle_command(
        _state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            CommandA::Inner => Ok(vec![EventA::Inner]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }
}

impl AggregateManager for AggregateA {
    type EventStore = PgStore<Self>;

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

pub struct AggregateB {
    event_store: PgStore<Self>,
}

impl AggregateB {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    pub async fn new_store(pool: &Pool<Postgres>) -> Result<PgStore<Self>, CounterError> {
        let projectors: Vec<Box<dyn Projector<Self> + Send + Sync>> = vec![Box::new(CounterProjector)];

        PgStore::new(pool, projectors, vec![]).setup().await
    }
}

impl Aggregate for AggregateB {
    type State = ();
    type Command = CommandB;
    type Event = EventB;
    type Error = CounterError;

    fn name() -> &'static str {
        "b"
    }

    fn handle_command(
        _state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            CommandB::Inner => Ok(vec![EventB::Inner]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }
}

impl AggregateManager for AggregateB {
    type EventStore = PgStore<Self>;

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}
