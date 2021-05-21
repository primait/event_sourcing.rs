use esrs::aggregate::AggregateName;
use esrs::policy::PgPolicy;
use esrs::projector::PgProjector;
use esrs::sqlx::{Pool, Postgres};
use esrs::store::PgStore;

use crate::credit_card::aggregate::CreditCardAggregate;
use crate::credit_card::error::CreditCardError;
use crate::credit_card::event::CreditCardEvent;
use crate::credit_card::policy::BankAccountPolicy;
use crate::credit_card::projector::CreditCardsProjector;

pub struct CreditCardStore;

#[allow(clippy::new_ret_no_self)]
impl CreditCardStore {
    pub async fn new(pool: &Pool<Postgres>) -> Result<PgStore<CreditCardEvent, CreditCardError>, CreditCardError> {
        let projectors: Vec<Box<dyn PgProjector<CreditCardEvent, CreditCardError> + Send + Sync>> =
            vec![Box::new(CreditCardsProjector)];

        let policies: Vec<Box<dyn PgPolicy<CreditCardEvent, CreditCardError> + Send + Sync>> =
            vec![Box::new(BankAccountPolicy)];

        PgStore::new(pool, CreditCardAggregate::name(), projectors, policies).await
    }
}
