use uuid::Uuid;

use esrs::aggregate::{Aggregate, AggregateState};
use esrs::sqlx::postgres::PgPoolOptions;
use esrs::sqlx::{Pool, Postgres};
use esrs::store::PgStore;
use payments::bank_account::aggregate::BankAccountAggregate;
use payments::bank_account::command::BankAccountCommand;
use payments::bank_account::error::BankAccountError;
use payments::bank_account::event::BankAccountEvent;
use payments::bank_account::state::BankAccountState;
use payments::bank_account::store::BankAccountStore;
use payments::credit_card::aggregate::CreditCardAggregate;
use payments::credit_card::command::CreditCardCommand;
use payments::credit_card::error::CreditCardError;
use payments::credit_card::event::CreditCardEvent;
use payments::credit_card::state::CreditCardState;
use payments::credit_card::store::CreditCardStore;

#[actix_rt::main]
async fn main() {
    println!("\n======================================================== START\n");

    let connection_string: &str = "postgres://postgres:postgres@postgres:5432/postgres";

    let pool: Pool<Postgres> = PgPoolOptions::new()
        .connect(connection_string)
        .await
        .expect("Failed to create pool");

    let () = sqlx::migrate!("examples/payments/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let bank_account_id: Uuid = Uuid::new_v4();

    // Credit card
    let credit_card_store: PgStore<CreditCardEvent, CreditCardError> = CreditCardStore::new(&pool).await.unwrap();
    let credit_card_aggregate: CreditCardAggregate = CreditCardAggregate::new(credit_card_store);
    let credit_card_state: AggregateState<CreditCardState> = AggregateState::new(bank_account_id);

    let bank_account_store: PgStore<BankAccountEvent, BankAccountError> = BankAccountStore::new(&pool).await.unwrap();
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
