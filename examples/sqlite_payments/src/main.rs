use uuid::Uuid;

use esrs::aggregate::{Aggregate, AggregateState};
use esrs::sqlx;
use esrs::sqlx::sqlite::SqlitePoolOptions;
use esrs::sqlx::{Pool, Sqlite};
use esrs::store::SqliteStore;
use sqlite_payments::bank_account::aggregate::BankAccountAggregate;
use sqlite_payments::bank_account::command::BankAccountCommand;
use sqlite_payments::bank_account::error::BankAccountError;
use sqlite_payments::bank_account::event::BankAccountEvent;
use sqlite_payments::bank_account::state::BankAccountState;
use sqlite_payments::bank_account::store::BankAccountStore;
use sqlite_payments::credit_card::aggregate::CreditCardAggregate;
use sqlite_payments::credit_card::command::CreditCardCommand;
use sqlite_payments::credit_card::error::CreditCardError;
use sqlite_payments::credit_card::event::CreditCardEvent;
use sqlite_payments::credit_card::state::CreditCardState;
use sqlite_payments::credit_card::store::CreditCardStore;

#[tokio::main(threaded_scheduler)]
async fn main() {
    println!("\n======================================================== START\n");

    let args: Vec<String> = std::env::args().collect();
    println!("#### ARGS\n\n{}\n\n####\n", args.join("\n"));

    // First arg is something like `target/debug/examples/sqlite-payments`
    let connection_string: &str = &args[1..].first().map(|v| v.as_str()).unwrap_or("sqlite::memory:");

    let pool: Pool<Sqlite> = SqlitePoolOptions::new()
        .connect(connection_string)
        .await
        .expect("Failed to create pool");

    let () = sqlx::migrate!("examples/sqlite_payments/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let bank_account_id: Uuid = Uuid::new_v4();

    // Credit card
    let credit_card_store: SqliteStore<CreditCardEvent, CreditCardError> = CreditCardStore::new(&pool).await.unwrap();
    let credit_card_aggregate: CreditCardAggregate = CreditCardAggregate::new(credit_card_store);
    let credit_card_state: AggregateState<CreditCardState> = AggregateState::new(bank_account_id);

    let bank_account_store: SqliteStore<BankAccountEvent, BankAccountError> =
        BankAccountStore::new(&pool).await.unwrap();
    let bank_account_aggregate: BankAccountAggregate = BankAccountAggregate::new(bank_account_store);
    let bank_account_state: AggregateState<BankAccountState> =
        AggregateState::new_with_state(bank_account_id, BankAccountState::default());

    // Salary deposit (1000)
    let bank_account_state: AggregateState<BankAccountState> = bank_account_aggregate
        .handle_command(bank_account_state, BankAccountCommand::Deposit { amount: 1000 })
        .await
        .unwrap();

    println!(
        "===> Your bank account balance is {} euros",
        bank_account_state.inner().balance
    );

    let tv_price: i32 = 599;
    println!(
        "===> You are buying a new TV using your credit card. Price is {} euros",
        tv_price
    );

    let _credit_card_state: AggregateState<CreditCardState> = credit_card_aggregate
        .handle_command(credit_card_state, CreditCardCommand::Pay { amount: tv_price })
        .await
        .unwrap();

    let bank_account_state: AggregateState<BankAccountState> = bank_account_aggregate
        .load(bank_account_id)
        .await
        .expect("Failed to load bank account state");

    println!(
        "===> Now your bank account balance is {} euros",
        bank_account_state.inner().balance
    );

    println!("\n======================================================== FINISHED\n")
}
