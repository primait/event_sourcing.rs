use sqlx::migrate::MigrateDatabase;
use sqlx::{pool::PoolOptions, Pool, Postgres};
use uuid::Uuid;

use customize_persistence_flow::{aggregate::CounterAggregate, projector::Counter, structs::CounterCommand};
use esrs::aggregate::{AggregateManager, AggregateState};

#[tokio::main]
async fn main() {
    let database_url: String = std::env::var("DATABASE_URL").expect("DATABASE_URL variable not set");

    Postgres::drop_database(database_url.as_str()).await.unwrap();
    Postgres::create_database(database_url.as_str()).await.unwrap();

    let pool: Pool<Postgres> = PoolOptions::new()
        .connect(database_url.as_str())
        .await
        .expect("Failed to create pool");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let count_id = Uuid::new_v4();

    // Construct the aggregation, and some nil state for it
    let aggregate = CounterAggregate::new(&pool)
        .await
        .expect("Failed to construct aggregate");
    let state = AggregateState::new(count_id);

    // Increment counter once
    let state = aggregate
        .handle(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");

    // Retrieve counter projection from database and print
    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");
    println!("Count is: {}", counter.count);
    assert!(counter.count == 1);

    // Increment counter twice
    let state = aggregate
        .handle(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");
    let state = aggregate
        .handle(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");

    // Retrieve counter projection from database and print
    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");
    println!("Count is: {}", counter.count);
    assert!(counter.count == 3);

    // Decrement counter once
    let _state = aggregate
        .handle(state, CounterCommand::Decrement)
        .await
        .expect("Failed to handle increment command");

    // Retrieve counter projection from database and print
    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");
    println!("Count is: {}", counter.count);
    assert!(counter.count == 2);
}
