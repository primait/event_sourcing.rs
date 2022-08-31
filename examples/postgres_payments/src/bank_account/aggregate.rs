use async_trait::async_trait;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState, Eraser};
use esrs::projector::PgProjectorEraser;
use esrs::store::{EraserStore, EventStore, PgStore};

use crate::bank_account::command::BankAccountCommand;
use crate::bank_account::error::BankAccountError;
use crate::bank_account::event::BankAccountEvent;
use crate::bank_account::projector::BankAccountProjector;
use crate::bank_account::state::BankAccountState;

pub type BankAccountStore = PgStore<
    BankAccountEvent,
    BankAccountError,
    dyn PgProjectorEraser<BankAccountEvent, BankAccountError> + Send + Sync,
>;

pub struct BankAccountAggregate {
    event_store: BankAccountStore,
}

impl BankAccountAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, BankAccountError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    pub async fn new_store(pool: &Pool<Postgres>) -> Result<BankAccountStore, BankAccountError> {
        let projectors: Vec<Box<dyn PgProjectorEraser<BankAccountEvent, BankAccountError> + Send + Sync>> =
            vec![Box::new(BankAccountProjector)];

        PgStore::new::<Self>(pool, projectors, vec![]).await
    }
}

#[async_trait]
impl Eraser<BankAccountEvent, BankAccountError> for BankAccountAggregate {
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), BankAccountError> {
        self.event_store.delete(aggregate_id).await
    }
}

impl Aggregate for BankAccountAggregate {
    type State = BankAccountState;
    type Command = BankAccountCommand;
    type Event = BankAccountEvent;
    type Error = BankAccountError;

    fn name() -> &'static str {
        "bank_account"
    }

    fn handle_command(
        aggregate_state: &AggregateState<BankAccountState>,
        cmd: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match cmd {
            BankAccountCommand::Withdraw { amount } if amount < 0 => Err(Self::Error::NegativeAmount),
            BankAccountCommand::Withdraw { amount } if aggregate_state.inner().balance - amount < 0 => {
                Err(Self::Error::NegativeBalance)
            }
            BankAccountCommand::Withdraw { amount } => Ok(vec![BankAccountEvent::Withdrawn { amount }]),
            BankAccountCommand::Deposit { amount } if amount < 0 => Err(Self::Error::NegativeAmount),
            BankAccountCommand::Deposit { amount } => Ok(vec![BankAccountEvent::Deposited { amount }]),
        }
    }

    fn apply_event(state: BankAccountState, event: &Self::Event) -> BankAccountState {
        match event {
            BankAccountEvent::Withdrawn { amount } => state.sub_amount(*amount),
            BankAccountEvent::Deposited { amount } => state.add_amount(*amount),
        }
    }
}

impl AggregateManager for BankAccountAggregate {
    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }
}
