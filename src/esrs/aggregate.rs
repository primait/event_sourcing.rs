use std::fmt::Debug;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::esrs::state::AggregateState;
use crate::esrs::store::{EventStore, StoreEvent};

/// The Identifier trait is responsible for naming an aggregate type.
/// Each aggregate type should have an identifier that is unique among all the aggregate types in your application.
///
/// Aggregates are linked to their instances & events using their `name` and their `aggregate_id`.  Be very careful when changing
/// `name`, as doing so will break the link between all the aggregates of their type, and their events!
pub trait Identifier {
    /// Returns the aggregate name
    fn name() -> &'static str
    where
        Self: Sized;
}

#[async_trait]
/// The Eraser trait is responsible for erasing an aggregate instance from history.
pub trait Eraser<
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
>
{
    /// `delete` should either complete the aggregate instance, along with all its associated events, or fail.
    /// If the deletion succeeds only partially, it _must_ return an error.
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Error>;
}

/// The AggregateManager is responsible for loading an aggregate from the store, mapping commands to events, and
/// persisting those events in the store. Be careful when implementing this trait, as you will be responsible for
/// threading AggregateState/Commands/Events correctly.  For example, a bad implementation could result in an AggregateState
/// that could not be replicated on load.
///
/// Unless you need to perform side effects as part of your command handling/verification you should implement the
/// safer `Aggregate` trait instead.
#[async_trait]
pub trait AggregateManager: Identifier + Aggregate {
    /// Returns the event store, configured for the aggregate
    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync);

    /// This function applies the event onto the aggregate and returns a new one, updated with the event data
    fn apply_event(id: &Uuid, state: Self::State, event: &StoreEvent<Self::Event>) -> Self::State {
        <Self as Aggregate>::apply_event(state, event.payload())
    }

    /// Validation should reject any command is inconsistent with the current aggregate state, or would result
    /// in one or more events that could not be applied onto the aggregate state.
    fn validate_command(aggregate_state: &AggregateState<Self::State>, cmd: &Self::Command) -> Result<(), Self::Error> {
        <Self as Aggregate>::validate_command(aggregate_state, cmd)
    }

    async fn handle_command(
        &self,
        aggregate_state: AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        // If cmd is valid...
        <Self as AggregateManager>::validate_command(&aggregate_state, &cmd)?;

        // ...let the aggregate handle it, in a pure way...
        let events = Aggregate::handle_command(self, &aggregate_state, cmd);

        // ...and then persist to store
        match AggregateManager::persist(self, &aggregate_state, events).await {
            Ok(events) => {
                match self.event_store().run_policies(&events).await {
                    Ok(_) => {
                        // all good
                    },
                    Err(_) => todo!("POLICIES FAILED, MAYBE LOG"),
                };
                let new_state = Self::apply_events(aggregate_state, events);
                Ok(new_state)
            },
            Err(_) => todo!("FAILED AND ROLLED BACK THE EVENTS"),
        }
    }

    /// Responsible for applying events in order onto the aggregate state, and incrementing the sequence number.
    /// You should avoid implementing this method, and be _very_ careful if you decide to do so.
    ///
    /// `events` will be passed in order of ascending sequence number.
    fn apply_events(
        aggregate_state: AggregateState<Self::State>,
        events: Vec<StoreEvent<Self::Event>>,
    ) -> AggregateState<Self::State> {
        let aggregate_id: &Uuid = &aggregate_state.id;
        let inner: Self::State = events.iter().fold(
            aggregate_state.inner,
            |acc: Self::State, event: &StoreEvent<Self::Event>| <Self as AggregateManager>::apply_event(aggregate_id, acc, event),
        );

        AggregateState {
            inner,
            sequence_number: events.last().map_or(0, |e| e.sequence_number()),
            ..aggregate_state
        }
    }

    /// Loads an aggregate instance from the event store, by applying previously persisted events onto
    /// the aggregate state by order of their sequence number
    async fn load(&self, aggregate_id: Uuid) -> Option<AggregateState<Self::State>> {
        let events: Vec<StoreEvent<Self::Event>> = self
            .event_store()
            .by_aggregate_id(aggregate_id)
            .await
            .ok()?
            .into_iter()
            .collect();

        if events.is_empty() {
            None
        } else {
            Some(Self::apply_events(AggregateState::new(aggregate_id), events))
        }
    }

    /// Persists an event into the event store - recording it in the aggregate instance's history.
    async fn persist(
        &self,
        aggregate_state: &AggregateState<Self::State>,
        events: Vec<Self::Event>,
    ) -> Result<Vec<StoreEvent<Self::Event>>, Self::Error> {
        self
            .event_store()
            .persist(aggregate_state.id, events, aggregate_state.next_sequence_number())
            .await

        // self.event_store().run_policies(&events).await?;

        // Ok(Self::apply_events(aggregate_state, events))
    }
}

/// The Aggregate trait is responsible for validating commands, mapping commands to events, and applying
/// events onto the aggregate state.
///
/// An Aggregate should be able to derive its own state from nothing but its initial configuration, and its
/// event stream. Applying the same events, in the same order, to the same aggregate, should always yield an
/// identical aggregate state.
///
/// This trait is purposefully _synchronous_.  If you are implementing this trait, your aggregate
/// should not have any side effects.  If you need additional information to handle commands correctly, then
/// consider either looking up that information and placing it in the command, or if that is not an option
/// you can implement the more powerful _asynchronous_ `AggregateManager` trait - it will then be your responsibility to
/// uphold the Aggregate _invariants_.
pub trait Aggregate {
    type State: Default + Clone + Debug + Send + Sync;
    type Command: Send + Sync;
    type Event: Serialize + DeserializeOwned + Send + Sync;
    type Error: Send + Sync;

    /// Validates a command against the current aggregate state.  The aggregate must be able to handle the command
    /// if validation succeeds.
    fn validate_command(aggregate_state: &AggregateState<Self::State>, cmd: &Self::Command) -> Result<(), Self::Error>;

    /// Handles a validated command, and emits events.
    fn handle_command(&self, aggregate_state: &AggregateState<Self::State>, cmd: Self::Command) -> Vec<Self::Event>;

    /// Updates the aggregate state using the new event.
    fn apply_event(state: Self::State, event: &Self::Event) -> Self::State;
}
