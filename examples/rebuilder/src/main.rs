use std::fmt::Debug;

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState};
use esrs::projector::PgProjector;
use serde::de::DeserializeOwned;
use serde::Serialize;
use simple_projection::aggregate::CounterAggregate;
use simple_projection::projector::{Counter, CounterProjector};
use simple_projection::structs::*;
use sqlx::migrate::MigrateDatabase;
use sqlx::{pool::PoolOptions, Pool, Postgres};
use uuid::Uuid;

// Rebuild the projection of a single aggregation, given the aggregate, an aggregate ID, a projector to rebuild and a pool connection
// This rebuilds the projection for all aggregate ids in a single transaction. An alternative (see _rebuild_per_id, below) is
// to rebuild on a per-id basis.
async fn rebuild_all_at_once<E, Err, A>(
    aggregate: &A,
    ids: Vec<Uuid>,
    projector: &dyn PgProjector<E, Err>,
    pool: &Pool<Postgres>,
) where
    A: AggregateManager<Event = E, Error = Err>,
    E: Serialize + DeserializeOwned + Send + Sync,
    Err: Debug,
{
    let mut events = Vec::new();
    for id in ids {
        events.append(
            &mut aggregate
                .event_store()
                .by_aggregate_id(id)
                .await
                .expect("failed to retrieve events"),
        );
    }
    let mut transaction = pool.begin().await.unwrap();
    for event in events {
        projector
            .project(&event, &mut transaction)
            .await
            .expect("Failed to project event");
    }
    transaction.commit().await.unwrap();
}

// An alternative approach to rebuilding that rebuilds the projected table for a given projection one
// aggregate ID at a time, rather than committing the entire table all at once
async fn _rebuild_per_id<E, Err, A>(
    aggregate: &A,
    ids: Vec<Uuid>,
    projector: &dyn PgProjector<E, Err>,
    pool: &Pool<Postgres>,
) where
    A: AggregateManager<Event = E, Error = Err>,
    <A as Aggregate>::Error: Debug,
    E: Serialize + DeserializeOwned + Send + Sync,
{
    for id in ids {
        rebuild_all_at_once(aggregate, vec![id], projector, pool).await;
    }
}

// Rebuild a number of boxed projectors at once, for a single aggregate, for a number of aggregate ids
async fn _rebuild_multiple_projectors<'a, E, Err, A>(
    aggregate: &'a A,
    ids: Vec<Uuid>,
    projectors: Vec<Box<dyn PgProjector<E, Err>>>,
    pool: &'a Pool<Postgres>,
) where
    A: AggregateManager<Event = E, Error = Err>,
    <A as Aggregate>::Error: Debug,
    E: Serialize + DeserializeOwned + Send + Sync,
{
    for projector in projectors {
        for id in &ids {
            rebuild_all_at_once(aggregate, vec![*id], projector.as_ref(), pool).await;
        }
    }
}

// A simple example demonstrating rebuilding a read-side projection from an event
// stream
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

    // Increment counter three times
    let state = aggregate
        .handle(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");
    let state = aggregate
        .handle(state, CounterCommand::Increment)
        .await
        .expect("Failed to handle increment command");
    let _state = aggregate
        .handle(state, CounterCommand::Increment)
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

    let projector = CounterProjector {};
    rebuild_all_at_once(&aggregate, vec![count_id], &projector, &pool).await;

    // Assert the counter has been rebuilt
    let res = Counter::by_id(count_id, &pool)
        .await
        .expect("Query failed")
        .expect("counter not found");
    assert!(res.counter_id == count_id && res.count == 3);
}
