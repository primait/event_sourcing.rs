use sqlx::migrate::MigrateDatabase;
use sqlx::{pool::PoolOptions, Pool, Postgres};
use uuid::Uuid;

use esrs::{AggregateManager, AggregateState, EventStore};

use crate::{aggregate::LoggingAggregate, structs::LoggingCommand};

pub mod aggregate;
pub mod structs;

#[tokio::main]
async fn main() {
    let database_url: String = std::env::var("DATABASE_URL").expect("DATABASE_URL variable not set");

    Postgres::drop_database(database_url.as_str()).await.unwrap();
    Postgres::create_database(database_url.as_str()).await.unwrap();

    let pool: Pool<Postgres> = PoolOptions::new()
        .connect(database_url.as_str())
        .await
        .expect("Failed to create pool");

    let logger_id = Uuid::new_v4();

    // Construct the aggregation, and some null state for it
    let aggregate = LoggingAggregate::new(&pool)
        .await
        .expect("Failed to construct aggregate");
    let state = AggregateState::new(logger_id);

    // Log some messages
    let state = aggregate
        .handle_command(state, LoggingCommand::TryLog(String::from("First logging message")))
        .await
        .expect("Failed to log message");

    // Due to how the saga pattern is implemented (with policies passing commands to the aggregate during another
    // commands handling), the state we get back is always invalid, so we need to retrieve it from the DB.
    // To demonstrate, 2 events exist in the event store, but if we check the state we get back from our call to
    // handle_command, it only shows 1 event as applied. Loading our state from the DB again, we see the correct
    // value of 2:
    let events = aggregate
        .event_store()
        .by_aggregate_id(logger_id)
        .await
        .expect("Failed to get events");

    assert!(events.len() == 2); // 2 events in the store, 1 from our command and 1 from the policy
    assert!(*state.inner() == 1); // However, the state we get back only has 1 event applied (it isn't valid)
    let state = aggregate.load(logger_id).await.expect("Failed to load state");
    assert!(*state.inner() == 2); // On loading the state from the DB, we get the correct number of applied events

    // Now we can use the newly loaded state to log another message, but we drop the invalid returned state
    let _ = aggregate
        .handle_command(state, LoggingCommand::TryLog(String::from("Second logging message")))
        .await
        .expect("Failed to log message");
}
