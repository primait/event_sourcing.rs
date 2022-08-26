use sqlx::{Pool, Postgres};

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState, Identifier};
use esrs::policy::PgPolicy;
use esrs::projector::PgProjector;
use esrs::store::{EventStore, PgStore};

use crate::credit_card::command::CreditCardCommand;
use crate::credit_card::error::CreditCardError;
use crate::credit_card::event::CreditCardEvent;
use crate::credit_card::policy::BankAccountPolicy;
use crate::credit_card::projector::CreditCardsProjector;
use crate::credit_card::state::CreditCardState;

const PAYMENT: &str = "credit_card";

pub struct CreditCardAggregate {
    event_store: PgStore<CreditCardEvent, CreditCardError>,
}

impl CreditCardAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CreditCardError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    pub async fn new_store(
        pool: &Pool<Postgres>,
    ) -> Result<PgStore<CreditCardEvent, CreditCardError>, CreditCardError> {
        let projectors: Vec<Box<dyn PgProjector<CreditCardEvent, CreditCardError> + Send + Sync>> =
            vec![Box::new(CreditCardsProjector)];

        let policies: Vec<Box<dyn PgPolicy<CreditCardEvent, CreditCardError> + Send + Sync>> =
            vec![Box::new(BankAccountPolicy)];

        PgStore::new::<Self>(pool, projectors, policies).await
    }
}

impl Identifier for CreditCardAggregate {
    fn name() -> &'static str {
        PAYMENT
    }
}

impl Aggregate for CreditCardAggregate {
    type State = CreditCardState;
    type Command = CreditCardCommand;
    type Event = CreditCardEvent;
    type Error = CreditCardError;

    fn handle_command(
        aggregate_state: &AggregateState<CreditCardState>,
        cmd: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match cmd {
            CreditCardCommand::Pay { amount } if amount < 0 => Err(Self::Error::NegativeAmount),
            CreditCardCommand::Pay { amount }
                if aggregate_state.inner().total_amount + amount > aggregate_state.inner().ceiling =>
            {
                Err(Self::Error::CeilingLimitReached)
            }
            CreditCardCommand::Pay { amount } => Ok(vec![CreditCardEvent::Payed { amount }]),
            CreditCardCommand::Refund { amount } if amount < 0 => Err(Self::Error::NegativeAmount),
            CreditCardCommand::Refund { amount } => Ok(vec![CreditCardEvent::Refunded { amount }]),
        }
    }

    // No state for credit_card aggregate
    fn apply_event(state: CreditCardState, event: &Self::Event) -> CreditCardState {
        match event {
            CreditCardEvent::Payed { amount } => state.add_amount(*amount),
            CreditCardEvent::Refunded { amount } => state.sub_amount(*amount),
        }
    }
}

impl AggregateManager for CreditCardAggregate {
    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }
}
