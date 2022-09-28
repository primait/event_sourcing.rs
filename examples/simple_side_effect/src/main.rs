use sqlx::migrate::MigrateDatabase;
use sqlx::{pool::PoolOptions, Pool, Postgres};
use uuid::Uuid;

use esrs::{AggregateManager, AggregateState};

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
        .handle_command(state, LoggingCommand::Log(String::from("First logging message")))
        .await
        .expect("Failed to log message");
    let state = aggregate
        .handle_command(state, LoggingCommand::Log(String::from("Second logging message")))
        .await
        .expect("Failed to log message");
    let state = aggregate
        .handle_command(state, LoggingCommand::Log(String::from("Third logging message")))
        .await
        .expect("Failed to log message");

    // Lets cause a projection error, to illustrate the difference between
    // projection and policy errors, from an AggregateState perspective
    let res = aggregate
        .handle_command(
            state,
            LoggingCommand::Log(String::from("This will fail since it contains fail_projection")),
        )
        .await;
    assert!(res.is_err());
    // We now need to rebuild the event state - and we'll see that there are only 3 events
    let state = aggregate.load(logger_id).await.expect("Failed to load aggregate state");
    assert_eq!(*state.inner(), 3);

    // Now we'll cause a policy error. This error is silenced by the `AggregateManager::store_events`
    // actual impl. It is overridable
    let res = aggregate
        .handle_command(
            state,
            LoggingCommand::Log(String::from("This will fail since it contains fail_policy")),
        )
        .await;
    assert!(res.is_ok());

    // We now need to rebuild the event state - and we'll see that there are 4 events
    let state = aggregate.load(logger_id).await.expect("Failed to load aggregate state");
    assert_eq!(*state.inner(), 4);
}
