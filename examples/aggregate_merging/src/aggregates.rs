use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Postgres};

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState};
use esrs::projector::PgProjector;
use esrs::store::PgStore;

use crate::projectors::CounterProjector;
use crate::structs::{CommandA, CommandB, CounterError, EventA, EventB, ProjectorEvent};

// A store of events
type Store<E> = PgStore<E, CounterError>;

// We use a template here to make instantiating the near-identical
// AggregateA and AggregateB easier.
pub struct GenericAggregate<E: Send + Sync + Serialize + DeserializeOwned> {
    event_store: Store<E>,
}

impl<
        E: Send
            + Sync
            + Serialize
            + DeserializeOwned
            + Into<ProjectorEvent> // EventStore bounds
            + Clone, // for the CounterProjector
    > GenericAggregate<E>
where
    GenericAggregate<E>: Aggregate,
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

pub type AggregateA = GenericAggregate<EventA>;
pub type AggregateB = GenericAggregate<EventB>;

#[async_trait]
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
    type EventStore = Store<Self::Event>;

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

#[async_trait]
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
    type EventStore = Store<Self::Event>;

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}
