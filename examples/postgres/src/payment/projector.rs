use async_trait::async_trait;
use uuid::Uuid;

use esrs::projector::PgProjector;
use esrs::sqlx;
use esrs::sqlx::{Executor, Postgres, Transaction};
use esrs::store::StoreEvent;

use crate::payment::error::Error;
use crate::payment::event::PaymentEvent;

pub struct PaymentsProjector;

#[async_trait]
impl PgProjector<PaymentEvent, Error> for PaymentsProjector {
    async fn project<'c>(
        &self,
        event: &StoreEvent<PaymentEvent>,
        transaction: &mut Transaction<'c, Postgres>,
    ) -> Result<(), Error> {
        match event.payload {
            PaymentEvent::Payed { amount } => {
                Ok(Payment::insert(Uuid::new_v4().to_string(), "pay".to_string(), amount, transaction).await?)
            }
            PaymentEvent::Refunded { amount } => {
                Ok(Payment::insert(Uuid::new_v4().to_string(), "refund".to_string(), amount, transaction).await?)
            }
        }
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct Payment {
    pub payment_id: String,
    pub payment_type: String,
    pub amount: i32,
}

impl Payment {
    pub async fn by_payment_id<'a, 'b: 'a, E>(payment_id: String, executor: E) -> Result<Option<Self>, sqlx::Error>
        where
            E: Executor<'b, Database=Postgres>,
    {
        sqlx::query_as::<_, Self>("SELECT * FROM payments WHERE payment_id = $1")
            .bind(payment_id)
            .fetch_optional(executor)
            .await
    }

    pub async fn insert<'a, 'b: 'a, E>(
        payment_id: String,
        payment_type: String,
        amount: i32,
        executor: E,
    ) -> Result<(), sqlx::Error>
        where
            E: Executor<'b, Database=Postgres>,
    {
        sqlx::query_as::<_, Self>("INSERT INTO payments (payment_id, payment_type, amount) VALUES ($1, $2, $3)")
            .bind(payment_id)
            .bind(payment_type)
            .bind(amount)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn all<'a, 'b: 'a, E>(executor: E) -> Result<Vec<Self>, sqlx::Error>
        where
            E: Executor<'b, Database=Postgres>,
    {
        sqlx::query_as::<_, Self>("SELECT * FROM payments")
            .fetch_all(executor)
            .await
    }

    pub async fn truncate<'a, 'b: 'a, E>(executor: E) -> Result<u64, sqlx::Error>
        where
            E: Executor<'b, Database=Postgres>,
    {
        use sqlx::Done;
        sqlx::query("TRUNCATE TABLE payments")
            .execute(executor)
            .await
            .map(|c| c.rows_affected())
    }
}
