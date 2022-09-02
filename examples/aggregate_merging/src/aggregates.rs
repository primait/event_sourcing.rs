use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Postgres};

use esrs::aggregate::{AggregateManager, AggregateState, Identifier};
use esrs::projector::PgProjector;
use esrs::store::{EventStore, PgStore};

use crate::projectors::CounterProjector;
use crate::structs::{CommandA, CommandB, CounterError, EventA, EventB, ProjectorEvent};

const A_EVENTS: &str = "A";
const B_EVENTS: &str = "B";

// A store of events
type Store<E> = PgStore<E, CounterError>;

// We use a template here to make instantiating the near-identical
// AggregateA and AggregateB easier.
pub struct Aggregate<E: Send + Sync + Serialize + DeserializeOwned> {
    event_store: Store<E>,
}

impl<
        E: Send
            + Sync
            + Serialize
            + DeserializeOwned
            + Into<ProjectorEvent> // EventStore bounds
            + Clone, // for the CounterProjector
    > Aggregate<E>
where
    Aggregate<E>: Identifier,
{
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    pub async fn new_store(pool: &Pool<Postgres>) -> Result<Store<E>, CounterError> {
        // Any aggregate based off this template will project to the CounterProjector
        let projectors: Vec<Box<dyn PgProjector<E, CounterError> + Send + Sync>> = vec![Box::new(CounterProjector)];

        PgStore::new::<Self>(pool, projectors, vec![]).await
    }
}

pub type AggregateA = Aggregate<EventA>;
pub type AggregateB = Aggregate<EventB>;

impl Identifier for AggregateA {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        A_EVENTS
    }
}

impl Identifier for AggregateB {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        B_EVENTS
    }
}

#[async_trait]
impl esrs::aggregate::Aggregate for AggregateA {
    type State = ();
    type Command = CommandA;
    type Event = EventA;
    type Error = CounterError;

    fn handle_command(
        _state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            CommandA::Inner => Ok(vec![EventA::Inner]),
        }
    }

    fn apply_event(state: Self::State, _: &Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }
}

impl AggregateManager for AggregateA {
    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }
}

#[async_trait]
impl esrs::aggregate::Aggregate for AggregateB {
    type State = ();
    type Command = CommandB;
    type Event = EventB;
    type Error = CounterError;

    fn apply_event(state: Self::State, _: &Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }

    fn handle_command(
        _state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            CommandB::Inner => Ok(vec![EventB::Inner]),
        }
    }
}

impl AggregateManager for AggregateB {
    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }
}
