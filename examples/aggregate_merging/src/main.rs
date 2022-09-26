use sqlx::migrate::MigrateDatabase;
use sqlx::{pool::PoolOptions, Pool, Postgres};
use uuid::Uuid;

use aggregate_merging::aggregates::AggregateA;
use aggregate_merging::aggregates::AggregateB;
use aggregate_merging::projectors::Counter;
use aggregate_merging::structs::CommandA;
use aggregate_merging::structs::CommandB;
use esrs::AggregateManager;

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

    let shared_id: Uuid = Uuid::new_v4();

    // Construct the two aggregates
    let agg_a = AggregateA::new(&pool).await.expect("Failed to construct aggregate");
    let agg_b = AggregateB::new(&pool).await.expect("Failed to construct aggregate");

    let counter = Counter::by_id(shared_id, &pool)
        .await
        .expect("Failed to retrieve counter");

    assert!(counter.is_none());

    // Increment each count once
    let _ = agg_a
        .handle_command(Default::default(), CommandA::Inner { shared_id })
        .await
        .expect("Failed to handle command a");

    let counter = Counter::by_id(shared_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");

    println!("Count A is {} and count B is {}", counter.count_a, counter.count_b);
    assert!(counter.count_a == 1 && counter.count_b == 0);

    let _ = agg_b
        .handle_command(Default::default(), CommandB::Inner { shared_id })
        .await
        .expect("Failed to handle command b");

    // Retrieve counter projection from database and print
    let counter = Counter::by_id(shared_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");

    println!("Count A is {} and count B is {}", counter.count_a, counter.count_b);
    assert!(counter.count_a == 1 && counter.count_b == 1);
}
