use std::ops::DerefMut;

use async_trait::async_trait;
use sqlx::{Executor, Sqlite};
use uuid::Uuid;

use esrs::pool::Transaction;
use esrs::projector::{SqliteProjector, SqliteProjectorEraser};
use esrs::store::StoreEvent;

use crate::bank_account::error::BankAccountError;
use crate::bank_account::event::BankAccountEvent;

pub struct BankAccountProjector;

#[async_trait]
impl SqliteProjector<BankAccountEvent, BankAccountError> for BankAccountProjector {
    async fn project<'c>(
        &self,
        event: &StoreEvent<BankAccountEvent>,
        transaction: &mut Transaction<'c, Sqlite>,
    ) -> Result<(), BankAccountError> {
        match event.payload {
            BankAccountEvent::Withdrawn { amount } => {
                let balance: Option<i32> = BankAccount::by_bank_account_id(event.aggregate_id, transaction.deref_mut())
                    .await?
                    .map(|bank_account| bank_account.balance);

                match balance {
                    Some(balance) => {
                        Ok(BankAccount::update(event.aggregate_id, balance - amount, transaction.deref_mut()).await?)
                    }
                    None => Ok(BankAccount::insert(event.aggregate_id, -amount, transaction.deref_mut()).await?),
                }
            }
            BankAccountEvent::Deposited { amount } => {
                let balance: Option<i32> = BankAccount::by_bank_account_id(event.aggregate_id, transaction.deref_mut())
                    .await?
                    .map(|bank_account| bank_account.balance);

                match balance {
                    Some(balance) => {
                        Ok(BankAccount::update(event.aggregate_id, balance + amount, transaction.deref_mut()).await?)
                    }
                    None => Ok(BankAccount::insert(event.aggregate_id, amount, transaction.deref_mut()).await?),
                }
            }
        }
    }
}

#[async_trait]
impl SqliteProjectorEraser<BankAccountEvent, BankAccountError> for BankAccountProjector {
    async fn delete<'c>(
        &self,
        aggregate_id: Uuid,
        transaction: &mut Transaction<'c, Sqlite>,
    ) -> Result<(), BankAccountError> {
        Ok(BankAccount::delete(aggregate_id, transaction.deref_mut()).await?)
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct BankAccount {
    pub bank_account_id: Uuid,
    pub balance: i32,
}

impl BankAccount {
    pub async fn by_bank_account_id(
        bank_account_id: Uuid,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM bank_accounts WHERE bank_account_id = $1")
            .bind(bank_account_id)
            .fetch_optional(executor)
            .await
    }

    pub async fn insert(
        bank_account_id: Uuid,
        balance: i32,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("INSERT INTO bank_accounts (bank_account_id, balance) VALUES ($1, $2)")
            .bind(bank_account_id)
            .bind(balance)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn update(
        bank_account_id: Uuid,
        balance: i32,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("UPDATE bank_accounts SET balance = $2 WHERE bank_account_id = $1")
            .bind(bank_account_id)
            .bind(balance)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn delete(
        bank_account_id: Uuid,
        executor: impl Executor<'_, Database = Sqlite>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("DELETE FROM bank_accounts WHERE bank_account_id = $1")
            .bind(bank_account_id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn all(executor: impl Executor<'_, Database = Sqlite>) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM bank_accounts")
            .fetch_all(executor)
            .await
    }

    pub async fn truncate(executor: impl Executor<'_, Database = Sqlite>) -> Result<u64, sqlx::Error> {
        sqlx::query("TRUNCATE TABLE bank_accounts")
            .execute(executor)
            .await
            .map(|c| c.rows_affected())
    }
}
