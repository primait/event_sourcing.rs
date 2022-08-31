use postgres_payments::bank_account::event::BankAccountEvent;
use sqlx::pool::PoolOptions;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::aggregate::{AggregateManager, AggregateState};
use postgres_payments::bank_account::aggregate::BankAccountAggregate;
use postgres_payments::bank_account::command::BankAccountCommand;
use postgres_payments::bank_account::state::BankAccountState;

#[tokio::test]
async fn postgres_credit_card_aggregate_event_order_test() {
    let connection_string: String = std::env::var("DATABASE_URL")
        .ok()
        .unwrap_or_else(|| "postgres://postgres:postgres@postgres:5432/postgres".to_string());

    let pool: Pool<Postgres> = PoolOptions::new()
        .connect(connection_string.as_str())
        .await
        .expect("Failed to create pool");

    let () = sqlx::migrate!("../migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let bank_account_id: Uuid = Uuid::new_v4();

    let mut bank_account_state: AggregateState<BankAccountState> =
        AggregateState::new_with_state(bank_account_id, BankAccountState::default());
    let bank_account_aggregate: BankAccountAggregate = BankAccountAggregate::new(&pool).await.unwrap();

    bank_account_state = bank_account_aggregate
        .handle(bank_account_state, BankAccountCommand::Deposit { amount: 1000 })
        .await
        .unwrap();

    bank_account_aggregate
        .handle(bank_account_state, BankAccountCommand::Withdraw { amount: 10 })
        .await
        .unwrap();

    let events = bank_account_aggregate
        .event_store()
        .by_aggregate_id(bank_account_id)
        .await
        .unwrap();

    assert!(matches!(events[0].payload(), BankAccountEvent::Deposited { .. }));

    let event_id = events[0].id;
    let first = serde_json::to_value(events[0].payload()).unwrap();

    sqlx::query("update bank_account_events set payload = $1 where id = $2")
        .bind(first)
        .bind(event_id)
        .execute(&pool)
        .await
        .unwrap();

    let events = bank_account_aggregate
        .event_store()
        .by_aggregate_id(bank_account_id)
        .await
        .unwrap();
    assert!(matches!(events[0].payload(), BankAccountEvent::Deposited { .. }));
}
