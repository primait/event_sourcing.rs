use std::fmt::Debug;

use aggregate_merging::aggregates::{AggregateA, AggregateB};
use aggregate_merging::projectors::{Counter, CounterProjector, self};
use aggregate_merging::structs::{CommandA, CommandB, EventA, EventB, ProjectorEvent, CounterError};
use esrs::aggregate::{AggregateManager, AggregateState};
use esrs::projector::SqliteProjector;
use esrs::store::{EventStore, StoreEvent};
use serde::de::DeserializeOwned;
use serde::Serialize;

use sqlx::{pool::PoolOptions, Pool, Sqlite};
use uuid::Uuid;

// A simple example demonstrating rebuilding a read-side projection from an event
// stream
#[tokio::main]
async fn main() {
    let pool: Pool<Sqlite> = PoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create pool");

    let count_id = Uuid::new_v4();

    setup(&pool, count_id.clone()).await;
    let agg_a = AggregateA::new(&pool).await.unwrap();
    let agg_b = AggregateB::new(&pool).await.unwrap();
    let store_a = agg_a.event_store();
    let store_b = agg_b.event_store();


    let ids = vec![count_id];
    let projectors: Vec<Box<dyn SqliteProjector<ProjectorEvent, CounterError>>> = vec![Box::new(CounterProjector)];
    for id in ids {
        // For a given ID, retrieve the event lists for both aggregates
        let events_a = store_a.by_aggregate_id(id).await.unwrap();
        let events_b = store_b.by_aggregate_id(id).await.unwrap();

        // Using StoreEvent::into, convert both event lists into a StoreEvent the shared projector can consume
        let mut events: Vec<StoreEvent<ProjectorEvent>> = events_a.into_iter().map(|a| a.into()).collect();
        events.extend(events_b.into_iter().map(|b| b.into()));

        // Since we've retrieved two different event lists and then concatenated them, we need to sort by timestamp
        // to ensure in-order projection
        events.sort_by_key(|e| e.occurred_on);
        // And project them
        for event in events {
            let mut connection = pool.acquire().await.unwrap();
            let _ = sqlx::query("BEGIN").execute(&mut connection).await.unwrap();
            for projector in &projectors {
                projector.project(&event, &mut connection).await.unwrap();
            }
            let _ = sqlx::query("COMMIT").execute(&mut connection).await.unwrap();
        }
    }

    let retrieved = Counter::by_id(count_id, &pool).await.unwrap().unwrap();
    assert!(retrieved.count_a == 1 && retrieved.count_b == 1);

}


async fn setup(pool: &Pool<Sqlite>, count_id: Uuid) {
    let () = sqlx::migrate!("./migrations")
    .run(pool)
    .await
    .expect("Failed to run migrations");

    // Construct the two aggregates
    let agg_a = AggregateA::new(pool).await.expect("Failed to construct aggregate");
    let a_state = AggregateState::new(count_id);

    let agg_b = AggregateB::new(pool).await.expect("Failed to construct aggregate");
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

    //Drop and rebuild the counters projection table
    sqlx::query("DROP TABLE counters")
        .execute(pool)
        .await
        .expect("Failed to drop table");
    sqlx::query("CREATE TABLE counters (\"counter_id\" UUID PRIMARY KEY NOT NULL, \"count_a\" INTEGER NOT NULL, \"count_b\" INTEGER NOT NULL );")
        .execute(pool)
        .await
        .expect("Failed to recreate counters table");

}