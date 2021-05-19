use esrs_core::aggregate::{Aggregate, AggregateName};
use esrs_core::postgres::{PgProjector, PgStore};
use esrs_core::sqlx;
use esrs_core::sqlx::{Pool, Postgres};
use esrs_core::sqlx::postgres::PgPoolOptions;
use esrs_core::state::AggregateState;
use postgres_example::payment::aggregate::PaymentAggregate;
use postgres_example::payment::command::PaymentCommand;
use postgres_example::payment::error::Error;
use postgres_example::payment::event::PaymentEvent;
use postgres_example::payment::projector::{Payment, PaymentsProjector};
use postgres_example::payment::state::PaymentState;

#[actix_rt::test]
async fn check_amounts() {
    let connection_string: &str = "postgres://postgres:postgres@postgres:5432/postgres";

    let pool: Pool<Postgres> = PgPoolOptions::new()
        .connect(connection_string)
        .await
        .expect("Failed to create pool");

    let () = sqlx::migrate!("examples/postgres/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");
    let truncated_rows: u64 = Payment::truncate(&pool)
        .await
        .expect("Failed to truncate payments table");

    println!("Truncated {} rows", truncated_rows);

    let projectors: Vec<Box<dyn PgProjector<PaymentEvent, Error> + Send + Sync>> = vec![Box::new(PaymentsProjector {})];

    // Payment aggregate store
    let payment_store: PgStore<PaymentEvent, Error> =
        PgStore::new(&pool, PaymentAggregate::name(), projectors).await.unwrap();

    // Payment aggregate
    let payment_aggregate: PaymentAggregate = PaymentAggregate::new(payment_store);

    let state: AggregateState<PaymentState> = AggregateState::default();
    assert_eq!(state.inner().total_amount, 0);

    // Cannot pay negative amount
    let result = payment_aggregate
        .handle_command(state.clone(), PaymentCommand::Pay { amount: -10 })
        .await;

    assert_eq!(result.err().unwrap().to_string(), "Negative amount");

    // Cannot refund negative amount
    let result = payment_aggregate
        .handle_command(state.clone(), PaymentCommand::Refund { amount: -10 })
        .await;

    assert_eq!(result.err().unwrap().to_string(), "Negative amount");

    // Cannot refund until total amount become negative (0 - 10)
    let result = payment_aggregate
        .handle_command(state.clone(), PaymentCommand::Refund { amount: 10 })
        .await;

    assert_eq!(result.err().unwrap().to_string(), "Negative total amount");

    // Payment of 10 euros
    let state: AggregateState<PaymentState> = payment_aggregate
        .handle_command(state.clone(), PaymentCommand::Pay { amount: 10 })
        .await
        .unwrap();

    assert_eq!(state.inner().total_amount, 10);

    // Payment of 15 euros. Total amount is 25
    let state: AggregateState<PaymentState> = payment_aggregate
        .handle_command(state, PaymentCommand::Pay { amount: 15 })
        .await
        .unwrap();

    assert_eq!(state.inner().total_amount, 25);

    // Payment of 20 euros. Total amount is 45
    let state: AggregateState<PaymentState> = payment_aggregate
        .handle_command(state, PaymentCommand::Pay { amount: 20 })
        .await
        .unwrap();

    assert_eq!(state.inner().total_amount, 45);

    assert_eq!(
        state.inner().total_amount,
        payment_aggregate.load(*state.id()).await.unwrap().inner().total_amount
    );

    // Refund of 30 euros. Total amount is 15
    let state: AggregateState<PaymentState> = payment_aggregate
        .handle_command(state, PaymentCommand::Refund { amount: 30 })
        .await
        .unwrap();

    assert_eq!(state.inner().total_amount, 15);
    assert_eq!(
        state.inner().total_amount,
        payment_aggregate.load(*state.id()).await.unwrap().inner().total_amount
    );

    // 4 payments have been done
    let payments: Vec<Payment> = Payment::all(&pool).await.unwrap();

    assert_eq!(payments.len(), 4);
    assert!(payments
        .iter()
        .any(|payment| payment.amount == 10 && payment.payment_type == "pay"));
    assert!(payments
        .iter()
        .any(|payment| payment.amount == 15 && payment.payment_type == "pay"));
    assert!(payments
        .iter()
        .any(|payment| payment.amount == 20 && payment.payment_type == "pay"));
    assert!(payments
        .iter()
        .any(|payment| payment.amount == 30 && payment.payment_type == "refund"));
}
