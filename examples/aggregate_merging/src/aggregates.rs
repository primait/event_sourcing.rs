use sqlx::{Pool, Postgres};

use esrs::postgres::PgStore;
use esrs::{Aggregate, AggregateManager};

use crate::projectors::CounterTransactionalQuery;
use crate::structs::{CommandA, CommandB, CounterError, EventA, EventB};

// We use a template here to make instantiating the near-identical
// AggregateA and AggregateB easier.
pub struct AggregateA {
    pub event_store: PgStore<Self>,
}

impl AggregateA {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        let event_store: PgStore<AggregateA> = PgStore::new(pool.clone())
            .set_transactional_queries(vec![Box::new(CounterTransactionalQuery)])
            .setup()
            .await?;

        Ok(Self { event_store })
    }
}

impl Aggregate for AggregateA {
    type State = ();
    type Command = CommandA;
    type Event = EventA;
    type Error = CounterError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            CommandA::Inner { shared_id } => Ok(vec![EventA::Inner { shared_id }]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }
}

impl AggregateManager for AggregateA {
    type EventStore = PgStore<Self>;

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "a"
    }

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

pub struct AggregateB {
    pub event_store: PgStore<Self>,
}

impl AggregateB {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        let event_store: PgStore<AggregateB> = PgStore::new(pool.clone())
            .set_transactional_queries(vec![Box::new(CounterTransactionalQuery)])
            .setup()
            .await?;

        Ok(Self { event_store })
    }
}

impl Aggregate for AggregateB {
    type State = ();
    type Command = CommandB;
    type Event = EventB;
    type Error = CounterError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            CommandB::Inner { shared_id } => Ok(vec![EventB::Inner { shared_id }]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }
}

impl AggregateManager for AggregateB {
    type EventStore = PgStore<Self>;

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "b"
    }

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}
