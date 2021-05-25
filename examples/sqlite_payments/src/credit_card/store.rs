use sqlx::{Pool, Sqlite};

use esrs::aggregate::AggregateName;
use esrs::policy::SqlitePolicy;
use esrs::projector::SqliteProjector;
use esrs::store::SqliteStore;

use crate::credit_card::aggregate::CreditCardAggregate;
use crate::credit_card::error::CreditCardError;
use crate::credit_card::event::CreditCardEvent;
use crate::credit_card::policy::BankAccountPolicy;
use crate::credit_card::projector::CreditCardsProjector;

pub struct CreditCardStore;

#[allow(clippy::new_ret_no_self)]
impl CreditCardStore {
    pub async fn new(pool: &Pool<Sqlite>) -> Result<SqliteStore<CreditCardEvent, CreditCardError>, CreditCardError> {
        let projectors: Vec<Box<dyn SqliteProjector<CreditCardEvent, CreditCardError> + Send + Sync>> =
            vec![Box::new(CreditCardsProjector)];

        let policies: Vec<Box<dyn SqlitePolicy<CreditCardEvent, CreditCardError> + Send + Sync>> =
            vec![Box::new(BankAccountPolicy)];

        SqliteStore::new(pool, CreditCardAggregate::name(), projectors, policies).await
    }
}
