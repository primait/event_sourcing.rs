use esrs::aggregate::{AggregateManager, AggregateState};
use sqlx::{pool::PoolOptions, Pool, Sqlite};
use uuid::Uuid;

use crate::{aggregate::CounterAggregate, projector::Counter, structs::CounterCommand};

pub mod aggregate;
pub mod projector;
pub mod structs;

#[tokio::main]
async fn main() {
    println!("Starting pool");
    let pool: Pool<Sqlite> = PoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create pool");

    println!("Running migrations");
    let () = sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    println!("Migrations run");

    let count_id = Uuid::new_v4();

    // Construct the aggregation, and some nil state for it
    let aggregate = CounterAggregate::new(&pool)
        .await
        .expect("Failed to construct aggregate");
    let state = AggregateState::new(count_id);

    // Increment counter once
    let state = aggregate
        .handle_command(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");

    // Retrieve counter projection from sqlite and print
    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");
    println!("Count is: {}", counter.count);
    assert!(counter.count == 1);

    // Increment counter twice
    let state = aggregate
        .handle_command(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");
    let state = aggregate
        .handle_command(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");

    // Retrieve counter projection from sqlite and print
    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");
    println!("Count is: {}", counter.count);
    assert!(counter.count == 3);

    // Decrement counter once
    let _state = aggregate
        .handle_command(state, CounterCommand::Decrement)
        .await
        .expect("Failed to handle increment command");

    // Retrieve counter projection from sqlite and print
    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");
    println!("Count is: {}", counter.count);
    assert!(counter.count == 2);
}
