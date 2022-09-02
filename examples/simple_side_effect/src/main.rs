use sqlx::{pool::PoolOptions, Pool, Sqlite};
use uuid::Uuid;

use esrs::aggregate::{AggregateManager, AggregateState};

use crate::{aggregate::LoggingAggregate, structs::LoggingCommand};

pub mod aggregate;
pub mod structs;

#[tokio::main]
async fn main() {
    let pool: Pool<Sqlite> = PoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create pool");

    let () = sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    let logger_id = Uuid::new_v4();

    // Construct the aggregation, and some null state for it
    let aggregate = LoggingAggregate::new(&pool)
        .await
        .expect("Failed to construct aggregate");
    let state = AggregateState::new(logger_id);

    // Log some messages
    let state = aggregate
        .handle(state, LoggingCommand::Log(String::from("First logging message")))
        .await
        .expect("Failed to log message");
    let state = aggregate
        .handle(state, LoggingCommand::Log(String::from("Second logging message")))
        .await
        .expect("Failed to log message");
    let state = aggregate
        .handle(state, LoggingCommand::Log(String::from("Third logging message")))
        .await
        .expect("Failed to log message");

    // Lets cause a projection error, to illustrate the difference between
    // projection and policy errors, from an AggregateState perspective
    let res = aggregate
        .handle(
            state,
            LoggingCommand::Log(String::from("This will fail since it contains fail_projection")),
        )
        .await;
    assert!(res.is_err());
    // We now need to rebuild the event state - and we'll see that there are only 3 events
    let state = aggregate.load(logger_id).await.expect("Failed to load aggregate state");
    assert!(*state.inner() == 3);

    // Now we'll cause a policy error. The event causing the error will be written to the
    // event store, so now when we reload the aggregate state, we'll see 4 events have
    // been applied
    let res = aggregate
        .handle(
            state,
            LoggingCommand::Log(String::from("This will fail since it contains fail_policy")),
        )
        .await;
    assert!(res.is_err());

    // We now need to rebuild the event state - and we'll see that there are 4 events
    let state = aggregate.load(logger_id).await.expect("Failed to load aggregate state");
    assert!(*state.inner() == 4);
}
