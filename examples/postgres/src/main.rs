use esrs::{Aggregate, AggregateState, Name, StoreParams};
use esrs::postgres::PgStore;
use esrs::sqlx::{Pool, Postgres};
use esrs::sqlx::postgres::PgPoolOptions;
use postgres::payment::aggregate::PaymentAggregate;
use postgres::payment::command::PaymentCommand;
use postgres::payment::error::Error;
use postgres::payment::event::PaymentEvent;
use postgres::payment::state::PaymentState;

#[actix_rt::main]
async fn main() {
    println!("\n======================================================== START\n");

    // Database connection params. This should exist in order to let this examples work.
    let connection_params: StoreParams = StoreParams {
        host: "localhost",
        port: None,
        user: "postgres",
        pass: "postgres",
        schema: "postgres",
    };

    // Can either crate it from pool
    let pool: Pool<Postgres> = PgPoolOptions::new()
        .connect(connection_params.postgres_url().as_str())
        .await
        .unwrap();

    // let

    // Payment aggregate store
    let payment_store: PgStore<PaymentEvent, Error> =
        PgStore::new(&pool, PaymentAggregate::name(), vec![]).await.unwrap();

    // Payment aggregate
    let payment_aggregate: PaymentAggregate = PaymentAggregate::new(payment_store);
    let state: AggregateState<PaymentState> = AggregateState::default();

    println!("Paying 10€..\n");

    // Payment of 10 euros
    let state: AggregateState<PaymentState> = payment_aggregate
        .handle_command(state, PaymentCommand::Pay { amount: 10 })
        .await
        .unwrap();

    println!("Your total payed amount is {}€\n", state.inner().total_amount);

    println!("Ops. You owe just 5€. Have to refund 5€!\n");

    // Refund of 10 euros. Total amount is 20
    let state: AggregateState<PaymentState> = payment_aggregate
        .handle_command(state, PaymentCommand::Refund { amount: 5 })
        .await
        .unwrap();
    println!("Your total payed amount is {}€\n", state.inner().total_amount);
}
