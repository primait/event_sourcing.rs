use async_trait::async_trait;
use uuid::Uuid;

use esrs::aggregate::{Aggregate, AggregateName, AggregateState};
use esrs::store::{EventStore, PgStore, StoreEvent};

use crate::credit_card::command::CreditCardCommand;
use crate::credit_card::error::CreditCardError;
use crate::credit_card::event::CreditCardEvent;
use crate::credit_card::state::CreditCardState;

const PAYMENT: &str = "credit_card";

pub struct CreditCardAggregate {
    event_store: PgStore<CreditCardEvent, CreditCardError>,
}

impl CreditCardAggregate {
    pub fn new(event_store: PgStore<CreditCardEvent, CreditCardError>) -> Self {
        Self { event_store }
    }
}

impl AggregateName for CreditCardAggregate {
    fn name() -> &'static str {
        PAYMENT
    }
}

#[async_trait]
impl Aggregate for CreditCardAggregate {
    type State = CreditCardState;
    type Command = CreditCardCommand;
    type Event = CreditCardEvent;
    type Error = CreditCardError;

    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }

    // No state for credit_card aggregate
    fn apply_event(_id: &Uuid, state: CreditCardState, event: &StoreEvent<Self::Event>) -> CreditCardState {
        match event.payload() {
            CreditCardEvent::Payed { amount } => state.add_amount(*amount),
            CreditCardEvent::Refunded { amount } => state.sub_amount(*amount),
        }
    }

    fn validate_command(
        aggregate_state: &AggregateState<CreditCardState>,
        cmd: &Self::Command,
    ) -> Result<(), Self::Error> {
        match cmd {
            // Cannot pay with negative amounts
            CreditCardCommand::Pay { amount } if *amount < 0 => Err(Self::Error::NegativeAmount),
            // Check if plafond limit has not been reached
            CreditCardCommand::Pay { amount }
                if aggregate_state.inner().total_amount + *amount > aggregate_state.inner().plafond =>
            {
                Err(Self::Error::PlafondLimitReached)
            }
            CreditCardCommand::Pay { .. } => Ok(()),
            // Cannot refund with negative amounts
            CreditCardCommand::Refund { amount } if *amount < 0 => Err(Self::Error::NegativeAmount),
            CreditCardCommand::Refund { .. } => Ok(()),
        }
    }

    async fn do_handle_command(
        &self,
        aggregate_state: AggregateState<CreditCardState>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        match cmd {
            CreditCardCommand::Pay { amount } => self.persist(aggregate_state, CreditCardEvent::Payed { amount }).await,
            CreditCardCommand::Refund { amount } => {
                self.persist(aggregate_state, CreditCardEvent::Refunded { amount })
                    .await
            }
        }
    }
}
