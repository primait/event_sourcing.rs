use async_trait::async_trait;
use uuid::Uuid;

use esrs::projector::PgProjector;
use esrs::store::StoreEvent;

use crate::bank_account::error::BankAccountError;
use crate::bank_account::event::BankAccountEvent;
use sqlx::{Executor, Postgres, Transaction};

pub struct BankAccountProjector;

#[async_trait]
impl PgProjector<BankAccountEvent, BankAccountError> for BankAccountProjector {
    async fn project<'c>(
        &self,
        event: &StoreEvent<BankAccountEvent>,
        transaction: &mut Transaction<'c, Postgres>,
    ) -> Result<(), BankAccountError> {
        match event.payload {
            BankAccountEvent::Withdrawn { amount } => {
                let balance: Option<i32> = get_balance(event.aggregate_id, transaction).await?;
                match balance {
                    Some(balance) => Ok(BankAccount::update(event.aggregate_id, balance - amount, transaction).await?),
                    None => Ok(BankAccount::insert(event.aggregate_id, 0, transaction).await?),
                }
            }
            BankAccountEvent::Deposited { amount } => {
                let balance: Option<i32> = get_balance(event.aggregate_id, transaction).await?;
                match balance {
                    Some(balance) => Ok(BankAccount::update(event.aggregate_id, balance + amount, transaction).await?),
                    None => Ok(BankAccount::insert(event.aggregate_id, 0, transaction).await?),
                }
            }
        }
    }
}

// Moved value `transaction` if trying to directly do this in handle_event. It seems that,
// using `E: Executor<'b, Database = Postgres>`, compiler is not able to understand that `transaction`
// is a mutable reference.
async fn get_balance(
    bank_account_id: Uuid,
    transaction: &mut Transaction<'_, Postgres>,
) -> Result<Option<i32>, BankAccountError> {
    Ok(BankAccount::by_bank_account_id(bank_account_id, transaction)
        .await?
        .map(|bank_account| bank_account.balance))
}

#[derive(sqlx::FromRow, Debug)]
pub struct BankAccount {
    pub bank_account_id: Uuid,
    pub balance: i32,
}

impl BankAccount {
    pub async fn by_bank_account_id<'a, 'b: 'a, E>(
        bank_account_id: Uuid,
        executor: E,
    ) -> Result<Option<Self>, sqlx::Error>
    where
        E: Executor<'b, Database = Postgres>,
    {
        sqlx::query_as::<_, Self>("SELECT * FROM bank_accounts WHERE bank_account_id = $1")
            .bind(bank_account_id)
            .fetch_optional(executor)
            .await
    }

    pub async fn insert<'a, 'b: 'a, E>(bank_account_id: Uuid, balance: i32, executor: E) -> Result<(), sqlx::Error>
    where
        E: Executor<'b, Database = Postgres>,
    {
        sqlx::query_as::<_, Self>("INSERT INTO bank_accounts (bank_account_id, balance) VALUES ($1, $2)")
            .bind(bank_account_id)
            .bind(balance)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn update<'a, 'b: 'a, E>(bank_account_id: Uuid, balance: i32, executor: E) -> Result<(), sqlx::Error>
    where
        E: Executor<'b, Database = Postgres>,
    {
        sqlx::query_as::<_, Self>("UPDATE bank_accounts SET balance = $2 WHERE bank_account_id = $1")
            .bind(bank_account_id)
            .bind(balance)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn all<'a, 'b: 'a, E>(executor: E) -> Result<Vec<Self>, sqlx::Error>
    where
        E: Executor<'b, Database = Postgres>,
    {
        sqlx::query_as::<_, Self>("SELECT * FROM bank_accounts")
            .fetch_all(executor)
            .await
    }

    pub async fn truncate<'a, 'b: 'a, E>(executor: E) -> Result<u64, sqlx::Error>
    where
        E: Executor<'b, Database = Postgres>,
    {
        use sqlx::Done;
        sqlx::query("TRUNCATE TABLE bank_accounts")
            .execute(executor)
            .await
            .map(|c| c.rows_affected())
    }
}
