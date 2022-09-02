use sqlx::{pool::PoolOptions, Pool, Postgres};
use sqlx::migrate::MigrateDatabase;
use uuid::Uuid;

use aggregate_merging::{
    aggregates::AggregateA,
    aggregates::AggregateB,
    projectors::Counter,
    structs::{CommandA, CommandB},
};
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

    let () = sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let count_id = Uuid::new_v4();

    // Construct the two aggregates
    let agg_a = AggregateA::new(&pool).await.expect("Failed to construct aggregate");
    let a_state = AggregateState::new(count_id);

    let agg_b = AggregateB::new(&pool).await.expect("Failed to construct aggregate");
    let b_state = AggregateState::new(count_id);

    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter");

    assert!(counter.is_none());

    // Increment each count once
    let _ = agg_a
        .handle(a_state, CommandA::Inner)
        .await
        .expect("Failed to handle command a");

    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");

    println!("Count A is {} and count B is {}", counter.count_a, counter.count_b);
    assert!(counter.count_a == 1 && counter.count_b == 0);

    let _ = agg_b
        .handle(b_state, CommandB::Inner)
        .await
        .expect("Failed to handle command b");

    // Retrieve counter projection from database and print
    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");

    println!("Count A is {} and count B is {}", counter.count_a, counter.count_b);
    assert!(counter.count_a == 1 && counter.count_b == 1);
}
