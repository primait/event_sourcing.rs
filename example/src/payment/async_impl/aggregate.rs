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
        aggregate_state: &mut AggregateState<PaymentState>,
        event: &StoreEvent<Self::Event>,
    ) -> AggregateState<PaymentState> {
        match event.payload() {
            PaymentEvent::Payed { amount } => {
                aggregate_state.inner_mut().add_amount(*amount);
                aggregate_state
            }
            PaymentEvent::Refunded { amount } => {
                aggregate_state.inner_mut().sub_amount(*amount);
                aggregate_state
            }
        }
        .to_owned()
    }

    fn validate_command(aggregate_state: &AggregateState<PaymentState>, cmd: &Self::Command) -> Result<(), Self::Error> {
        match cmd {
            PaymentCommand::Pay { amount } if *amount < 0.0 => Err(Self::Error::NegativeAmount),
            PaymentCommand::Pay { .. } => Ok(()),
            PaymentCommand::Refund { amount } if *amount < 0.0 => Err(Self::Error::NegativeAmount),
            PaymentCommand::Refund { amount } if aggregate_state.inner().total_amount - *amount < 0.0 => {
                Err(Self::Error::NegativeTotalAmount)
            }
            PaymentCommand::Refund { .. } => Ok(()),
        }
    }

    async fn do_handle_command(
        &self,
        aggregate_state: &mut AggregateState<PaymentState>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        match cmd {
            PaymentCommand::Pay { amount } => self.persist(aggregate_state, PaymentEvent::Payed { amount }).await,
            PaymentCommand::Refund { amount } => self.persist(aggregate_state, PaymentEvent::Refunded { amount }).await,
        }
    }
}
