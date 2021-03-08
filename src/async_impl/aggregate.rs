use async_trait::async_trait;
use serde::de::DeserializeOwned;
use serde::Serialize;
use uuid::Uuid;

use crate::async_impl::store::EventStore;
use crate::async_impl::store::StoreEvent;
use crate::state::AggregateState;
use crate::{IdentifiableAggregate, SequenceNumber};

/// Aggregate trait. It is used to keep the state in-memory and to validate commands. It also persist events
#[async_trait]
pub trait Aggregate: IdentifiableAggregate {
    type State: Default + Clone + Send + Sync;
    type Command: Send + Sync;
    type Event: Serialize + DeserializeOwned + Clone + Send + Sync;
    type Error: Send + Sync;

    /// Event store configured for aggregate
    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync);

    /// This function applies the event onto the aggregate and returns a new one, updated with the event data
    fn apply_event(id: &Uuid, state: Self::State, event: &StoreEvent<Self::Event>) -> Self::State;

    fn validate_command(aggregate_state: &AggregateState<Self::State>, cmd: &Self::Command) -> Result<(), Self::Error>;

    async fn do_handle_command(
        &self,
        aggregate_state: AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error>;

    async fn handle_command(
        &self,
        aggregate_state: AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        Self::validate_command(&aggregate_state, &cmd)?;
        self.do_handle_command(aggregate_state, cmd).await
    }

    fn apply_events(
        aggregate_state: AggregateState<Self::State>,
        events: Vec<StoreEvent<Self::Event>>,
    ) -> AggregateState<Self::State> {
        let mut max_seq_number: SequenceNumber = 0;

        let inner: Self::State = events.iter().fold(
            aggregate_state.inner.to_owned(),
            |acc: Self::State, event: &StoreEvent<Self::Event>| {
                if event.sequence_number() > max_seq_number {
                    max_seq_number = event.sequence_number()
                }
                Self::apply_event(&aggregate_state.id, acc, event)
            },
        );

        AggregateState {
            inner,
            sequence_number: max_seq_number,
            ..aggregate_state
        }
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
            Some(Self::apply_events(AggregateState::new(aggregate_id), events))
        }
    }

    async fn persist(
        &self,
        aggregate_state: AggregateState<Self::State>,
        event: Self::Event,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        let next_sequence_number: SequenceNumber = aggregate_state.sequence_number + 1;
        Ok(self
            .event_store()
            .persist(aggregate_state.id, event, next_sequence_number)
            .await
            .map(|event| AggregateState {
                inner: Self::apply_event(&aggregate_state.id.to_owned(), aggregate_state.inner, &event),
                sequence_number: next_sequence_number,
                ..aggregate_state
            })?)
    }
}
