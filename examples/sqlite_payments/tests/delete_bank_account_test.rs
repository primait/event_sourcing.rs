use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use esrs::aggregate::{Aggregate, AggregateState, Eraser};
use esrs::store::SqliteStore;
use sqlite_payments::bank_account::aggregate::BankAccountAggregate;
use sqlite_payments::bank_account::command::BankAccountCommand;
use sqlite_payments::bank_account::error::BankAccountError;
use sqlite_payments::bank_account::event::BankAccountEvent;
use sqlite_payments::bank_account::projector::BankAccount;
use sqlite_payments::bank_account::state::BankAccountState;
use sqlite_payments::bank_account::store::BankAccountStore;

#[tokio::test(threaded_scheduler)]
async fn sqlite_delete_bank_account_aggregate_and_read_model_test() {
    let connection_string: &str = "sqlite::memory:";

    let pool: Pool<Sqlite> = SqlitePoolOptions::new()
        .connect(connection_string)
        .await
        .expect("Failed to create pool");

    let () = sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let bank_account_id: Uuid = Uuid::new_v4();
    let bank_account_state: AggregateState<BankAccountState> =
        AggregateState::new_with_state(bank_account_id, BankAccountState::default());
    let bank_account_store: SqliteStore<BankAccountEvent, BankAccountError> =
        BankAccountStore::new(&pool).await.unwrap();
    let bank_account_aggregate: BankAccountAggregate = BankAccountAggregate::new(bank_account_store);

    // Salary deposit (1000)
    let bank_account_state: AggregateState<BankAccountState> = bank_account_aggregate
        .handle_command(bank_account_state, BankAccountCommand::Deposit { amount: 1000 })
        .await
        .unwrap();

    assert_eq!(
        bank_account_state.inner().balance,
        bank_account_aggregate
            .load(*bank_account_state.id())
            .await
            .unwrap()
            .inner()
            .balance
    );

    let bank_account: BankAccount = BankAccount::by_bank_account_id(bank_account_id, &pool)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(bank_account.balance, 1000);

    bank_account_aggregate
        .delete(bank_account_id)
        .await
        .expect("Failed to delete aggregate and its read models");

    let bank_account_opt: Option<BankAccount> = BankAccount::by_bank_account_id(bank_account_id, &pool).await.unwrap();
    assert!(bank_account_opt.is_none());
}