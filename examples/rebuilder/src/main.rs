use esrs::aggregate::{AggregateManager, AggregateState};
use esrs::projector::SqliteProjector;
use sqlx::{pool::PoolOptions, Pool, Sqlite};
use uuid::Uuid;

use crate::{aggregate::CounterAggregate, projector::Counter, structs::CounterCommand};

pub mod aggregate;
pub mod projector;
pub mod structs;

// A simple example demonstrating rebuilding a read-side projection from an event
// stream
#[tokio::main]
async fn main() {
    println!("Starting pool");
    let pool: Pool<Sqlite> = PoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create pool");

    let () = sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let count_id = Uuid::new_v4();

    // Construct the aggregation, and some nil state for it
    let aggregate = CounterAggregate::new(&pool)
        .await
        .expect("Failed to construct aggregate");
    let state = AggregateState::new(count_id);

    // Increment counter three times
    let state = aggregate
        .handle_command(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");
    let state = aggregate
        .handle_command(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");
    let _state = aggregate
        .handle_command(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");

    //Drop and rebuild the counters table
    sqlx::query("DROP TABLE counters")
        .execute(&pool)
        .await
        .expect("Failed to drop table");
    sqlx::query("CREATE TABLE counters (\"counter_id\" UUID PRIMARY KEY NOT NULL, \"count\" INTEGER NOT NULL );")
        .execute(&pool)
        .await
        .expect("Failed to recreate counters table");

    // Assert the counter doesn't exist
    let res = Counter::by_id(count_id, &pool).await.expect("Query failed");
    assert!(res.is_none());

    // Rebuild the counter - scoped here to demonstrate that everything is dropped after and it's the underlying
    // sql table that has been updated
    {
        let events = aggregate
            .event_store()
            .by_aggregate_id(count_id)
            .await
            .expect("Failed to retrieve events");
        let projector = projector::CounterProjector {};
        let mut connection = pool.acquire().await.expect("Failed to acquire connection");
        let _ = sqlx::query("BEGIN")
            .execute(&mut connection)
            .await
            .expect("Failed to begin transaction");
        for event in events {
            projector
                .project(&event, &mut connection)
                .await
                .expect("Failed to project event");
        }
        let _ = sqlx::query("COMMIT")
            .execute(&mut connection)
            .await
            .expect("Failed to commit rebuild");
    }

    // Assert the counter has been rebuilt
    let res = Counter::by_id(count_id, &pool)
        .await
        .expect("Query failed")
        .expect("counter not found");
    assert!(res.counter_id == count_id && res.count == 3);
}
