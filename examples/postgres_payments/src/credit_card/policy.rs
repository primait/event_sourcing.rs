use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use esrs::aggregate::{Aggregate, AggregateState};
use esrs::policy::PgPolicy;
use esrs::store::StoreEvent;

use crate::bank_account::aggregate::BankAccountAggregate;
use crate::bank_account::command::BankAccountCommand;
use crate::bank_account::state::BankAccountState;
use crate::credit_card::error::CreditCardError;
use crate::credit_card::event::CreditCardEvent;

// Against a credit_card create new event to BankAccount aggregate to update balance
pub struct BankAccountPolicy;

#[async_trait]
impl PgPolicy<CreditCardEvent, CreditCardError> for BankAccountPolicy {
    async fn handle_event(
        &self,
        event: &StoreEvent<CreditCardEvent>,
        pool: &Pool<Postgres>,
    ) -> Result<(), CreditCardError> {
        let bank_account: BankAccountAggregate = BankAccountAggregate::new(pool).await?;

        let state: AggregateState<BankAccountState> = bank_account.load(event.aggregate_id).await.unwrap_or_default();

        let command: BankAccountCommand = match event.payload {
            CreditCardEvent::Payed { amount } => BankAccountCommand::Withdraw { amount },
            CreditCardEvent::Refunded { amount } => BankAccountCommand::Deposit { amount },
        };

        let _ = bank_account.handle_command(state, command).await?;

        Ok(())
    }
}
