use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::blocking::store::EventStore;
use crate::blocking::store::StoreEvent;
use crate::state::AggregateState;

/// Aggregate trait. It is used to keep the state in-memory and to validate commands. It also persist events
pub trait Aggregate {
    type State: Default + Clone;
    type Command;
    type Event: Serialize + DeserializeOwned + Clone;
    type Error;

    /// Event store configured for aggregate
    fn event_store(&self) -> &dyn EventStore<Self::Event, Self::Error>;

    /// This function applies the event onto the aggregate and returns a new one, updated with the event data
    fn apply_event(
        aggregate_state: &mut AggregateState<Self::State>,
        event: &StoreEvent<Self::Event>,
    ) -> AggregateState<Self::State>;

    fn validate_command(aggregate_state: &AggregateState<Self::State>, cmd: &Self::Command) -> Result<(), Self::Error>;

    fn do_handle_command(
        &self,
        aggregate_state: &mut AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error>;

    fn handle_command(
        &self,
        aggregate_state: &mut AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        Self::validate_command(aggregate_state, &cmd)?;
        self.do_handle_command(aggregate_state, cmd)
    }

    fn apply_events(
        aggregate_state: &mut AggregateState<Self::State>,
        events: Vec<StoreEvent<Self::Event>>,
    ) -> AggregateState<Self::State> {
        for event in events.iter() {
            Self::apply_event(aggregate_state.set_sequence_number(event.sequence_number()), event);
        }
        aggregate_state.to_owned()
    }

    fn load(&self, aggregate_id: Uuid) -> Option<AggregateState<Self::State>> {
        let events: Vec<StoreEvent<Self::Event>> = self
            .event_store()
            .by_aggregate_id(aggregate_id)
            .ok()?
            .into_iter()
            .collect();

        if events.is_empty() {
            None
        } else {
            Some(Self::apply_events(&mut AggregateState::new(aggregate_id), events))
        }
    }

    fn persist(
        &self,
        aggregate_state: &mut AggregateState<Self::State>,
        event: Self::Event,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        let new_aggregate_state = aggregate_state.incr_sequence_number();
        Ok(self
            .event_store()
            .persist(new_aggregate_state.id(), event, new_aggregate_state.sequence_number())
            .map(|event| Self::apply_event(new_aggregate_state, &event))?)
    }
}
