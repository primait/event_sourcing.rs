use esrs::aggregate::AggregateName;
use esrs::projector::PgProjector;
use esrs::sqlx::{Pool, Postgres};
use esrs::store::PgStore;

use crate::bank_account::aggregate::BankAccountAggregate;
use crate::bank_account::error::BankAccountError;
use crate::bank_account::event::BankAccountEvent;
use crate::bank_account::projector::BankAccountProjector;

pub struct BankAccountStore;

#[allow(clippy::new_ret_no_self)]
impl BankAccountStore {
    pub async fn new(pool: &Pool<Postgres>) -> Result<PgStore<BankAccountEvent, BankAccountError>, BankAccountError> {
        let projectors: Vec<Box<dyn PgProjector<BankAccountEvent, BankAccountError> + Send + Sync>> =
            vec![Box::new(BankAccountProjector)];

        PgStore::new(pool, BankAccountAggregate::name(), projectors, vec![]).await
    }
}
