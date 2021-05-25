use async_trait::async_trait;
use sqlx::{Executor, Sqlite, Transaction};
use uuid::Uuid;

use esrs::projector::SqliteProjector;
use esrs::store::StoreEvent;

use crate::credit_card::error::CreditCardError;
use crate::credit_card::event::CreditCardEvent;

pub struct CreditCardsProjector;

#[async_trait]
impl SqliteProjector<CreditCardEvent, CreditCardError> for CreditCardsProjector {
    async fn project<'c>(
        &self,
        event: &StoreEvent<CreditCardEvent>,
        transaction: &mut Transaction<'c, Sqlite>,
    ) -> Result<(), CreditCardError> {
        match event.payload {
            CreditCardEvent::Payed { amount } => {
                Ok(CreditCard::insert(event.id, event.aggregate_id, "pay".to_string(), amount, transaction).await?)
            }
            CreditCardEvent::Refunded { amount } => {
                Ok(CreditCard::insert(event.id, event.aggregate_id, "refund".to_string(), amount, transaction).await?)
            }
        }
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
    pub async fn by_payment_id<'a, 'b: 'a, E>(payment_id: Uuid, executor: E) -> Result<Option<Self>, sqlx::Error>
    where
        E: Executor<'b, Database = Sqlite>,
    {
        sqlx::query_as::<_, Self>("SELECT * FROM credit_cards WHERE payment_id = $1")
            .bind(payment_id)
            .fetch_optional(executor)
            .await
    }

    pub async fn by_credit_card_id<'a, 'b: 'a, E>(credit_card_id: Uuid, executor: E) -> Result<Vec<Self>, sqlx::Error>
    where
        E: Executor<'b, Database = Sqlite>,
    {
        sqlx::query_as::<_, Self>("SELECT * FROM credit_cards WHERE credit_card_id = $1")
            .bind(credit_card_id)
            .fetch_all(executor)
            .await
    }

    pub async fn insert<'a, 'b: 'a, E>(
        payment_id: Uuid,
        credit_card_id: Uuid,
        credit_card_payment_type: String,
        amount: i32,
        executor: E,
    ) -> Result<(), sqlx::Error>
    where
        E: Executor<'b, Database = Sqlite>,
    {
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

    pub async fn all<'a, 'b: 'a, E>(executor: E) -> Result<Vec<Self>, sqlx::Error>
    where
        E: Executor<'b, Database = Sqlite>,
    {
        sqlx::query_as::<_, Self>("SELECT * FROM credit_cards")
            .fetch_all(executor)
            .await
    }

    pub async fn truncate<'a, 'b: 'a, E>(executor: E) -> Result<u64, sqlx::Error>
    where
        E: Executor<'b, Database = Sqlite>,
    {
        use sqlx::Done;
        sqlx::query("TRUNCATE TABLE credit_cards")
            .execute(executor)
            .await
            .map(|c| c.rows_affected())
    }
}
