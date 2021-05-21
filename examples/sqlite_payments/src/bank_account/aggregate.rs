use async_trait::async_trait;
use uuid::Uuid;

use esrs::aggregate::{Aggregate, AggregateName, AggregateState};
use esrs::store::{EventStore, SqliteStore, StoreEvent};

use crate::bank_account::command::BankAccountCommand;
use crate::bank_account::error::BankAccountError;
use crate::bank_account::event::BankAccountEvent;
use crate::bank_account::state::BankAccountState;

const BANK_ACCOUNT: &str = "bank_account";

pub struct BankAccountAggregate {
    event_store: SqliteStore<BankAccountEvent, BankAccountError>,
}

impl BankAccountAggregate {
    pub fn new(event_store: SqliteStore<BankAccountEvent, BankAccountError>) -> Self {
        Self { event_store }
    }
}

impl AggregateName for BankAccountAggregate {
    fn name() -> &'static str {
        BANK_ACCOUNT
    }
}

#[async_trait]
impl Aggregate for BankAccountAggregate {
    type State = BankAccountState;
    type Command = BankAccountCommand;
    type Event = BankAccountEvent;
    type Error = BankAccountError;

    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }

    fn apply_event(_id: &Uuid, state: BankAccountState, event: &StoreEvent<Self::Event>) -> BankAccountState {
        match event.payload() {
            BankAccountEvent::Withdrawn { amount } => state.sub_amount(*amount),
            BankAccountEvent::Deposited { amount } => state.add_amount(*amount),
        }
    }

    fn validate_command(
        aggregate_state: &AggregateState<BankAccountState>,
        cmd: &Self::Command,
    ) -> Result<(), Self::Error> {
        match cmd {
            BankAccountCommand::Withdraw { amount } if *amount < 0 => Err(Self::Error::NegativeAmount),
            BankAccountCommand::Withdraw { amount } if aggregate_state.inner().balance - *amount < 0 => {
                Err(Self::Error::NegativeBalance)
            }
            BankAccountCommand::Withdraw { .. } => Ok(()),
            BankAccountCommand::Deposit { amount } if *amount < 0 => Err(Self::Error::NegativeAmount),
            BankAccountCommand::Deposit { .. } => Ok(()),
        }
    }

    async fn do_handle_command(
        &self,
        aggregate_state: AggregateState<BankAccountState>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        match cmd {
            BankAccountCommand::Withdraw { amount } => {
                self.persist(aggregate_state, BankAccountEvent::Withdrawn { amount })
                    .await
            }
            BankAccountCommand::Deposit { amount } => {
                self.persist(aggregate_state, BankAccountEvent::Deposited { amount })
                    .await
            }
        }
    }
}
