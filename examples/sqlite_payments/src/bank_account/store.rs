use sqlx::{Pool, Sqlite};

use esrs::aggregate::AggregateName;
use esrs::projector::SqliteProjector;
use esrs::store::SqliteStore;

use crate::bank_account::aggregate::BankAccountAggregate;
use crate::bank_account::error::BankAccountError;
use crate::bank_account::event::BankAccountEvent;
use crate::bank_account::projector::BankAccountProjector;

pub struct BankAccountStore;

#[allow(clippy::new_ret_no_self)]
impl BankAccountStore {
    pub async fn new(pool: &Pool<Sqlite>) -> Result<SqliteStore<BankAccountEvent, BankAccountError>, BankAccountError> {
        let projectors: Vec<Box<dyn SqliteProjector<BankAccountEvent, BankAccountError> + Send + Sync>> =
            vec![Box::new(BankAccountProjector)];

        SqliteStore::new(pool, BankAccountAggregate::name(), projectors, vec![]).await
    }
}
