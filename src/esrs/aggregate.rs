use std::fmt::Debug;

use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::esrs::state::AggregateState;
use crate::esrs::store::{EventStore, StoreEvent};
use crate::types::SequenceNumber;

/// The Aggregate trait is responsible for validating commands, mapping commands to events, and applying
/// events onto the state.
///
/// An Aggregate should be able to derive its own state from nothing but its initial configuration, and its
/// event stream. Applying the same events, in the same order, to the same aggregate, should always yield an
/// identical aggregate state.
///
/// This trait is purposefully _synchronous_. If you are implementing this trait, your aggregate
/// should not have any side effects. If you need additional information to handle commands correctly, then
/// consider looking up that information and placing it in the command.
pub trait Aggregate {
    type State: Default + Clone + Debug + Send + Sync;
    type Command: Send + Sync;
    type Event: Serialize + DeserializeOwned + Send + Sync;
    type Error: Send + Sync;

    /// The `name` function is responsible for naming an aggregate type.
    /// Each aggregate type should have a name that is unique among all the aggregate types in your application.
    ///
    /// Aggregates are linked to their instances & events using their `name` and their `aggregate_id`.  Be very careful when changing
    /// `name`, as doing so will break the link between all the aggregates of their type, and their events!
    fn name() -> &'static str
    where
        Self: Sized;

    /// Handles, validate a command and emits events.
    fn handle_command(
        state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error>;

    /// Updates the aggregate state using the new event. This assumes that the event can be correctly applied
    /// to the state.
    ///
    /// If this is not the case, this function is allowed to panic.
    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State;
}

/// The AggregateManager is responsible for coupling the Aggregate with a Store, so that the events
/// can be persisted when handled, and the state can be reconstructed by loading and apply events sequentially.
///
/// It comes batteries-included, as you only need to implement the `event_store` getter. The basic API is:
/// 1. execute_command
/// 2. load
/// The other functions are used internally, but can be overridden if needed.
#[async_trait]
pub trait AggregateManager: Aggregate {
    type EventStore: EventStore<Self::Event, Self::Error> + Send + Sync;

    /// Returns the event store, configured for the aggregate
    fn event_store(&self) -> &Self::EventStore;

    /// Validates and handles the command onto the given state, and then passes the events to the store.
    async fn handle(
        &self,
        aggregate_state: AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        let events: Vec<Self::Event> = Self::handle_command(&aggregate_state, command)?;
        let stored_events: Vec<StoreEvent<Self::Event>> = self.store_events(&aggregate_state, events).await?;

        Ok(Self::apply_events(aggregate_state, stored_events))
    }

    /// Responsible for applying events in order onto the aggregate state, and incrementing the sequence number.
    ///
    /// `events` will be passed in order of ascending sequence number.
    ///
    /// You should _avoid_ implementing this function, and be _very_ careful if you decide to do so.
    fn apply_events(
        aggregate_state: AggregateState<Self::State>,
        events: Vec<StoreEvent<Self::Event>>,
    ) -> AggregateState<Self::State> {
        let sequence_number: SequenceNumber = events.last().map_or(0, StoreEvent::sequence_number);
        let inner: Self::State = events.into_iter().fold(
            aggregate_state.inner,
            |acc: Self::State, event: StoreEvent<Self::Event>| Self::apply_event(acc, event.payload),
        );

        AggregateState {
            inner,
            sequence_number,
            ..aggregate_state
        }
    }

    /// Loads an aggregate instance from the event store, by applying previously persisted events onto
    /// the aggregate state by order of their sequence number
    ///
    /// You should _avoid_ implementing this function, and be _very_ careful if you decide to do so.
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

    /// Transactional persists events in store - recording it in the aggregate instance's history.
    /// The store will also project the events. If an error occurs whilst persisting the events,
    /// the whole transaction is rolled back and the error is returned.
    ///
    /// The policies associated to the store are run here. A failure at this point will be silently
    /// ignored, and the new state returned successfully anyway.
    ///
    /// You should _avoid_ implementing this function, and be _very_ careful if you decide to do so.
    /// The only scenario where this function needs to be overwritten is if you need to change the
    /// behaviour of policies, e.g. if you want to log something on error.
    async fn store_events(
        &self,
        aggregate_state: &AggregateState<Self::State>,
        events: Vec<Self::Event>,
    ) -> Result<Vec<StoreEvent<Self::Event>>, Self::Error> {
        self.event_store()
            .persist(aggregate_state.id, events, aggregate_state.next_sequence_number())
            .await
    }
}

/// The Eraser trait is responsible for erasing an aggregate instance from history.
/// TODO: better name for this?
#[async_trait]
pub trait Eraser<Event, Error>
where
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
{
    /// `delete` should either complete the aggregate instance, along with all its associated events, or fail.
    /// If the deletion succeeds only partially, it _must_ return an error.
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Error>;
}
