use async_trait::async_trait;
use uuid::Uuid;

use crate::types::SequenceNumber;
use crate::{AggregateState, EventStore, StoreEvent};

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
    /// Internal aggregate state. This will be wrapped in `AggregateState` and could be used to validate
    /// commands.
    type State: Default + Clone + Send + Sync;

    /// A command is an action that the caller can execute over an aggregate in order to let it emit
    /// an event.
    type Command: Send;

    /// An event represents a fact that took place in the domain. They are the source of truth;
    /// your current state is derived from the events.
    type Event: Send + Sync;

    /// This associated type is used to get domain errors while handling a command.
    type Error;

    /// Handles, validate a command and emits events.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the user of this library set up command validations. Every error here
    /// could be just a "domain error". No technical errors.
    fn handle_command(state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error>;

    /// Updates the aggregate state using the new event. This assumes that the event can be correctly applied
    /// to the state.
    ///
    /// If this is not the case, this function is allowed to panic.
    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State;

    /// Updates the aggregate state using the list of new events. Take a look to
    /// [`Aggregate::apply_event`] for further information.
    fn apply_events(state: Self::State, events: Vec<Self::Event>) -> Self::State {
        events.into_iter().fold(state, |acc: Self::State, event: Self::Event| {
            Self::apply_event(acc, event)
        })
    }
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
    type EventStore: EventStore<Manager = Self> + Send + Sync;

    /// The `name` function is responsible for naming an aggregate type.
    /// Each aggregate type should have a name that is unique among all the aggregate types in your application.
    ///
    /// Aggregates are linked to their instances & events using their `name` and their `aggregate_id`.  Be very careful when changing
    /// `name`, as doing so will break the link between all the aggregates of their type, and their events!
    fn name() -> &'static str
    where
        Self: Sized;

    /// Returns the event store, configured for the aggregate
    fn event_store(&self) -> &Self::EventStore;

    /// Validates and handles the command onto the given state, and then passes the events to the store.
    async fn handle_command(
        &self,
        mut aggregate_state: AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        let events: Vec<Self::Event> = <Self as Aggregate>::handle_command(aggregate_state.inner(), command)?;
        let stored_events: Vec<StoreEvent<Self::Event>> = self.store_events(&mut aggregate_state, events).await?;

        Ok(<Self as AggregateManager>::apply_events(aggregate_state, stored_events))
    }

    /// Responsible for applying events in order onto the aggregate state, and incrementing the sequence number.
    ///
    /// `events` will be passed in order of ascending sequence number.
    ///
    /// You should _avoid_ implementing this function, and be _very_ careful if you decide to do so.
    fn apply_events(
        aggregate_state: AggregateState<Self::State>,
        store_events: Vec<StoreEvent<Self::Event>>,
    ) -> AggregateState<Self::State> {
        let sequence_number: SequenceNumber = store_events.last().map_or(0, StoreEvent::sequence_number);

        let events: Vec<Self::Event> = store_events
            .into_iter()
            .map(|store_event| store_event.payload)
            .collect();

        let inner: Self::State = <Self as Aggregate>::apply_events(aggregate_state.inner, events);

        AggregateState {
            sequence_number,
            inner,
            ..aggregate_state
        }
    }

    /// Loads an aggregate instance from the event store, by applying previously persisted events onto
    /// the aggregate state by order of their sequence number
    ///
    /// You should _avoid_ implementing this function, and be _very_ careful if you decide to do so.
    async fn load(&self, aggregate_id: impl Into<Uuid> + Send) -> Option<AggregateState<Self::State>> {
        let aggregate_id: Uuid = aggregate_id.into();

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
            Some(<Self as AggregateManager>::apply_events(
                AggregateState::new(aggregate_id),
                events,
            ))
        }
    }

    /// Acquires a lock on this aggregate instance, and only then loads it from the event store,
    /// by applying previously persisted events onto the aggregate state by order of their sequence number.
    ///
    /// The lock is contained in the returned `AggregateState`, and released when this is dropped.
    /// It can also be extracted with the `take_lock` method for more advanced uses.
    ///
    /// You should _avoid_ implementing this function, and be _very_ careful if you decide to do so.
    async fn lock_and_load(&self, aggregate_id: impl Into<Uuid> + Send) -> Option<AggregateState<Self::State>> {
        let id = aggregate_id.into();
        let Ok(guard) = self.event_store().lock(id).await else {
            return None;
        };
        self.load(id).await.map(|mut state| {
            state.set_lock(guard);
            state
        })
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
        aggregate_state: &mut AggregateState<Self::State>,
        events: Vec<Self::Event>,
    ) -> Result<Vec<StoreEvent<Self::Event>>, Self::Error> {
        self.event_store().persist(aggregate_state, events).await
    }

    /// `delete` should either complete the aggregate instance, along with all its associated events
    /// and projections, or fail.
    ///
    /// If the deletion succeeds only partially, it _must_ return an error.
    async fn delete(&self, aggregate_id: impl Into<Uuid> + Send) -> Result<(), Self::Error> {
        self.event_store().delete(aggregate_id.into()).await
    }
}
