use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::async_impl::store::EventStore;
use crate::async_impl::store::StoreEvent;
use crate::state::AggregateState;
use crate::Identifiable;

/// Aggregate trait. It is used to keep the state in-memory and to validate commands. It also persist events
#[async_trait]
pub trait Aggregate: Identifiable {
    type State: Default + Clone + Send + Sync;
    type Command: Send + Sync;
    type Event: Serialize + DeserializeOwned + Clone + Send + Sync;
    type Error: Send + Sync;

    /// Event store configured for aggregate
    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync);

    /// This function applies the event onto the aggregate and returns a new one, updated with the event data
    fn apply_event(
        aggregate_state: &mut AggregateState<Self::State>,
        event: &StoreEvent<Self::Event>,
    ) -> AggregateState<Self::State>;

    fn validate_command(aggregate_state: &AggregateState<Self::State>, cmd: &Self::Command) -> Result<(), Self::Error>;

    async fn do_handle_command(
        &self,
        aggregate_state: &mut AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error>;

    async fn handle_command(
        &self,
        aggregate_state: &mut AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        Self::validate_command(aggregate_state, &cmd)?;
        self.do_handle_command(aggregate_state, cmd).await
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
            Some(Self::apply_events(&mut AggregateState::new(aggregate_id), events))
        }
    }

    async fn persist(
        &self,
        aggregate_state: &mut AggregateState<Self::State>,
        event: Self::Event,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        let new_aggregate_state: &mut AggregateState<Self::State> = aggregate_state.incr_sequence_number();
        Ok(self
            .event_store()
            .persist(new_aggregate_state.id(), event, new_aggregate_state.sequence_number())
            .await
            .map(|event| Self::apply_event(new_aggregate_state, &event))?)
    }
}
