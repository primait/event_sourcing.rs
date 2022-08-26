use async_trait::async_trait;
use sqlx::{Pool, Sqlite};

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState, Identifier};
use esrs::policy::SqlitePolicy;
use esrs::projector::SqliteProjector;
use esrs::store::{EventStore, SqliteStore};

use crate::credit_card::command::CreditCardCommand;
use crate::credit_card::error::CreditCardError;
use crate::credit_card::event::CreditCardEvent;
use crate::credit_card::policy::BankAccountPolicy;
use crate::credit_card::projector::CreditCardsProjector;
use crate::credit_card::state::CreditCardState;

const PAYMENT: &str = "credit_card";

pub struct CreditCardAggregate {
    event_store: SqliteStore<CreditCardEvent, CreditCardError>,
}

impl CreditCardAggregate {
    pub async fn new(pool: &Pool<Sqlite>) -> Result<Self, CreditCardError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    pub async fn new_store(
        pool: &Pool<Sqlite>,
    ) -> Result<SqliteStore<CreditCardEvent, CreditCardError>, CreditCardError> {
        let projectors: Vec<Box<dyn SqliteProjector<CreditCardEvent, CreditCardError> + Send + Sync>> =
            vec![Box::new(CreditCardsProjector)];

        let policies: Vec<Box<dyn SqlitePolicy<CreditCardEvent, CreditCardError> + Send + Sync>> =
            vec![Box::new(BankAccountPolicy)];

        SqliteStore::new::<Self>(pool, projectors, policies).await
    }
}

impl Identifier for CreditCardAggregate {
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

    fn validate_command(
        aggregate_state: &AggregateState<CreditCardState>,
        cmd: &Self::Command,
    ) -> Result<(), Self::Error> {
        match cmd {
            // Cannot pay with negative amounts
            CreditCardCommand::Pay { amount } if *amount < 0 => Err(Self::Error::NegativeAmount),
            // Check if ceiling limit has not been reached
            CreditCardCommand::Pay { amount }
                if aggregate_state.inner().total_amount + *amount > aggregate_state.inner().ceiling =>
            {
                Err(Self::Error::CeilingLimitReached)
            }
            CreditCardCommand::Pay { .. } => Ok(()),
            // Cannot refund with negative amounts
            CreditCardCommand::Refund { amount } if *amount < 0 => Err(Self::Error::NegativeAmount),
            CreditCardCommand::Refund { .. } => Ok(()),
        }
    }

    fn handle_command(_aggregate_state: &AggregateState<CreditCardState>, cmd: Self::Command) -> Vec<Self::Event> {
        match cmd {
            CreditCardCommand::Pay { amount } => vec![CreditCardEvent::Payed { amount }],
            CreditCardCommand::Refund { amount } => vec![CreditCardEvent::Refunded { amount }],
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
