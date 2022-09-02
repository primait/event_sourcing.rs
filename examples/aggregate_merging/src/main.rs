use esrs::aggregate::{AggregateManager, AggregateState};
use sqlx::{pool::PoolOptions, Pool, Sqlite};
use uuid::Uuid;

use aggregate_merging::{
    aggregates::AggregateA,
    aggregates::AggregateB,
    projectors::Counter,
    structs::{CommandA, CommandB},
};

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

    // Construct the two aggregates
    let agg_a = AggregateA::new(&pool).await.expect("Failed to construct aggregate");
    let a_state = AggregateState::new(count_id);

    let agg_b = AggregateB::new(&pool).await.expect("Failed to construct aggregate");
    let b_state = AggregateState::new(count_id);

    // Increment each count once
    let _ = agg_a
        .handle_command(a_state, CommandA::Inner)
        .await
        .expect("Failed to handle command a");

    let _ = agg_b
        .handle_command(b_state, CommandB::Inner)
        .await
        .expect("Failed to handle command b");

    // Retrieve counter projection from sqlite and print
    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");
    assert!(counter.count_a == 1 && counter.count_b == 1);
}
