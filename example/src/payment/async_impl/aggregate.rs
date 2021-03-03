use async_trait::async_trait;
use esrs::aggregate::Aggregate;
use esrs::state::AggregateState;
use esrs::store::postgres::PostgreStore;
use esrs::store::{EventStore, StoreEvent};
use esrs::Identifiable;

use crate::payment::command::PaymentCommand;
use crate::payment::error::Error;
use crate::payment::event::PaymentEvent;
use crate::payment::state::PaymentState;

const PAYMENT: &str = "payment";

pub struct PaymentAggregate {
    event_store: PostgreStore<PaymentEvent, Error>,
}

impl PaymentAggregate {
    pub fn new(event_store: PostgreStore<PaymentEvent, Error>) -> Self {
        Self { event_store }
    }
}

impl Identifiable for PaymentAggregate {
    fn name() -> &'static str {
        PAYMENT
    }
}

#[async_trait]
impl Aggregate for PaymentAggregate {
    type State = PaymentState;
    type Command = PaymentCommand;
    type Event = PaymentEvent;
    type Error = Error;

    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }

    fn apply_event(
        aggregate_state: AggregateState<PaymentState>,
        event: &StoreEvent<Self::Event>,
    ) -> AggregateState<PaymentState> {
        match event.payload() {
            PaymentEvent::Payed { amount } => AggregateState {
                inner: PaymentState {
                    total_amount: aggregate_state.inner.total_amount + amount,
                },
                sequence_number: aggregate_state.sequence_number + 1,
                ..aggregate_state
            },
            PaymentEvent::Refunded { amount } => AggregateState {
                inner: PaymentState {
                    total_amount: aggregate_state.inner.total_amount - amount,
                },
                sequence_number: aggregate_state.sequence_number + 1,
                ..aggregate_state
            },
        }
    }

    fn validate_command(state: &AggregateState<PaymentState>, cmd: &Self::Command) -> Result<(), Self::Error> {
        match cmd {
            PaymentCommand::Pay { amount } if *amount < 0.0 => Err(Self::Error::NegativeAmount),
            PaymentCommand::Pay { .. } => Ok(()),
            PaymentCommand::Refund { amount } if *amount < 0.0 => Err(Self::Error::NegativeAmount),
            PaymentCommand::Refund { amount } if state.inner.total_amount - *amount < 0.0 => {
                Err(Self::Error::NegativeTotalAmount)
            }
            PaymentCommand::Refund { .. } => Ok(()),
        }
    }

    async fn do_handle_command(
        &self,
        state: AggregateState<PaymentState>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        match cmd {
            PaymentCommand::Pay { amount } => self.persist(state, PaymentEvent::Payed { amount }).await,
            PaymentCommand::Refund { amount } => self.persist(state, PaymentEvent::Refunded { amount }).await,
        }
    }
}
