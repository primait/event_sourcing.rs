use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use esrs::postgres::PgStore;
use esrs::{Aggregate, AggregateManager};

use crate::{a, b};
use crate::{Command, Error};

/// This is the `Aggregate` where we use the `a::Event`, with a versioned-event upcasting approach
pub struct AggregateA {
    event_store: PgStore<Self>,
}

impl AggregateA {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, Error> {
        let event_store: PgStore<Self> = PgStore::new(pool.clone()).setup().await?;
        Ok(Self { event_store })
    }
}

impl Aggregate for AggregateA {
    type State = ();
    type Command = Command;
    type Event = a::Event;
    type Error = Error;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::Increment { u } => Ok(vec![Self::Event::Incremented(a::IncPayload { u })]),
            Self::Command::Decrement { u } => Ok(vec![Self::Event::Decremented(a::DecPayload { u })]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        state
    }
}

#[async_trait]
impl AggregateManager for AggregateA {
    type EventStore = PgStore<Self>;
    fn name() -> &'static str {
        "a"
    }
    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

pub struct AggregateB {
    event_store: PgStore<Self>,
}

/// This is the `Aggregate` where we use the `EventB`, with a json upcasting approach
impl AggregateB {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, Error> {
        let event_store: PgStore<Self> = PgStore::new(pool.clone()).setup().await?;
        Ok(Self { event_store })
    }
}

impl Aggregate for AggregateB {
    type State = ();
    type Command = Command;
    type Event = b::Event;
    type Error = Error;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::Increment { u } => Ok(vec![Self::Event::Incremented(b::IncPayload { u })]),
            Self::Command::Decrement { u } => Ok(vec![Self::Event::Decremented(b::DecPayload { u })]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        state
    }
}

#[async_trait]
impl AggregateManager for AggregateB {
    type EventStore = PgStore<Self>;
    fn name() -> &'static str {
        "a"
    }
    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}
