use esrs::async_impl::aggregate::Aggregate;
use esrs::async_impl::store::postgres::{PgPoolOptions, Pool, PostgreStore, Postgres};
use esrs::state::AggregateState;
use esrs::{IdentifiableAggregate, StoreParams};

use example::payment::async_impl::PaymentAggregate;
use example::payment::command::PaymentCommand;
use example::payment::error::Error;
use example::payment::event::PaymentEvent;
use example::payment::state::PaymentState;

#[actix_rt::main]
async fn main() {
    println!("\n======================================================== START\n");

    // Database connection params. This should exist in order to let this example work.
    let connection_params: StoreParams = StoreParams {
        host: "localhost",
        port: None,
        user: "example",
        pass: "example",
        schema: "example",
    };

    // Can either crate it from pool
    let pool: Pool<Postgres> = PgPoolOptions::new().connect(connection_params.postgres_url().as_str()).await.unwrap();

    // Payment aggregate store
    let payment_store: PostgreStore<PaymentEvent, Error> = PostgreStore::new(&pool, PaymentAggregate::name(), vec![])
        .await
        .unwrap();

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
