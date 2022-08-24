use async_trait::async_trait;
use esrs::projector::SqliteProjector;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use esrs::aggregate::{AggregateManager, AggregateState, Identifier};
use esrs::store::{EventStore, SqliteStore, StoreEvent};

use crate::projectors::CounterProjector;
use crate::structs::{CommandA, CommandB, CounterError, EventA, EventB, ProjectorEvent};

const A_EVENTS: &str = "A";
const B_EVENTS: &str = "B";

// A store of events
type Store<E> = SqliteStore<E, CounterError>;

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
            + // EventStore bounds
            Into<ProjectorEvent>
            + Clone, // for the CounterProjector
    > Aggregate<E>
where
    Aggregate<E>: Identifier,
{
    pub async fn new(pool: &Pool<Sqlite>) -> Result<Self, CounterError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    pub async fn new_store(pool: &Pool<Sqlite>) -> Result<Store<E>, CounterError> {
        // Any aggregate based off this template will project to the CounterProjector
        let projectors: Vec<Box<dyn SqliteProjector<E, CounterError> + Send + Sync>> = vec![Box::new(CounterProjector)];

        SqliteStore::new::<Self>(pool, projectors, vec![]).await
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
impl AggregateManager for AggregateA {
    type State = ();
    type Command = CommandA;
    type Event = EventA;
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
        command: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        match command {
            CommandA::Inner => self.persist(aggregate_state, vec![EventA::Inner]).await,
        }
    }
}

#[async_trait]
impl AggregateManager for AggregateB {
    type State = ();
    type Command = CommandB;
    type Event = EventB;
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
        command: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        match command {
            CommandB::Inner => self.persist(aggregate_state, vec![EventB::Inner]).await,
        }
    }
}
