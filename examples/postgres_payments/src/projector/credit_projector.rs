use async_trait::async_trait;
use sqlx::pool::PoolConnection;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use esrs::projector::{PgProjector, PgProjectorEraser};
use esrs::store::StoreEvent;

use crate::error::CreditCardError;
use crate::event::CreditCardEvent;

pub struct CreditCardsProjector;

#[async_trait]
impl PgProjector<CreditCardEvent, CreditCardError> for CreditCardsProjector {
    async fn project(
        &self,
        event: &StoreEvent<CreditCardEvent>,
        connection: &mut PoolConnection<Postgres>,
    ) -> Result<(), CreditCardError> {
        match event.payload {
            CreditCardEvent::Payed { amount } => {
                Ok(CreditCard::insert(event.id, event.aggregate_id, "pay".to_string(), amount, connection).await?)
            }
            CreditCardEvent::Refunded { amount } => {
                Ok(CreditCard::insert(event.id, event.aggregate_id, "refund".to_string(), amount, connection).await?)
            }
        }
    }
}

#[async_trait]
impl PgProjectorEraser<CreditCardEvent, CreditCardError> for CreditCardsProjector {
    async fn delete(
        &self,
        aggregate_id: Uuid,
        connection: &mut PoolConnection<Postgres>,
    ) -> Result<(), CreditCardError> {
        Ok(CreditCard::delete(aggregate_id, connection).await?)
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct CreditCard {
    pub payment_id: Uuid,
    pub credit_card_id: Uuid,
    pub credit_card_payment_type: String,
    pub amount: i32,
}

impl CreditCard {
    pub async fn by_payment_id(
        payment_id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM credit_cards WHERE payment_id = $1")
            .bind(payment_id)
            .fetch_optional(executor)
            .await
    }

    pub async fn by_credit_card_id(
        credit_card_id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM credit_cards WHERE credit_card_id = $1")
            .bind(credit_card_id)
            .fetch_all(executor)
            .await
    }

    pub async fn insert(
        payment_id: Uuid,
        credit_card_id: Uuid,
        credit_card_payment_type: String,
        amount: i32,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>(
            "INSERT INTO credit_cards (payment_id, credit_card_id, credit_card_payment_type, amount) VALUES ($1, $2, $3, $4)",
        )
        .bind(payment_id)
        .bind(credit_card_id)
        .bind(credit_card_payment_type)
        .bind(amount)
        .fetch_optional(executor)
        .await
        .map(|_| ())
    }

    pub async fn delete(
        credit_card_id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query_as::<_, Self>("DELETE FROM credit_cards WHERE credit_card_id = $1")
            .bind(credit_card_id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn all(executor: impl Executor<'_, Database = Postgres>) -> Result<Vec<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM credit_cards")
            .fetch_all(executor)
            .await
    }

    pub async fn truncate(executor: impl Executor<'_, Database = Postgres>) -> Result<u64, sqlx::Error> {
        sqlx::query("TRUNCATE TABLE credit_cards")
            .execute(executor)
            .await
            .map(|c| c.rows_affected())
    }
}
