use std::convert::TryInto;
use std::fmt::Debug;

use chrono::{DateTime, Utc};
use futures_util::stream::StreamExt;
use serde_json::Value;
use sqlx::migrate::MigrateDatabase;
use sqlx::{pool::PoolOptions, Pool, Postgres, Transaction};
use uuid::Uuid;

use esrs::postgres::Projector;
use esrs::types::SequenceNumber;
use esrs::{Aggregate, AggregateManager, AggregateState, EventStore, StoreEvent};
use simple_projection::aggregate::CounterAggregate;
use simple_projection::projector::{Counter, CounterProjector};
use simple_projection::structs::*;

// Rebuild the projection of a single aggregation, given the aggregate, an aggregate ID, a list of
// projectors to rebuild and a pool connection this rebuilds the projections for all aggregate ids
// in a single transaction. An alternative (see _rebuild_per_id, below) is
// to rebuild on a per-id basis.
async fn rebuild_all_at_once<A>(
    events: Vec<StoreEvent<A::Event>>,
    projectors: &[Box<dyn Projector<A> + Send + Sync>],
    transaction: &mut Transaction<'_, Postgres>,
) where
    A: AggregateManager,
    <A as Aggregate>::Error: Debug,
{
    for event in events {
        for projector in projectors {
            projector
                .project(&event, &mut *transaction)
                .await
                .expect("Failed to project event");
        }
    }
}

// An alternative approach to rebuilding that rebuilds the projected table for a given projection one
// aggregate ID at a time, rather than committing the entire table all at once
async fn _rebuild_per_id<A>(
    aggregate: &A,
    ids: Vec<Uuid>,
    projectors: &[Box<dyn Projector<A> + Send + Sync>],
    pool: &Pool<Postgres>,
) where
    A: AggregateManager,
    <A as Aggregate>::Error: Debug,
{
    for id in ids {
        let mut transaction = pool.begin().await.unwrap();

        for projector in projectors {
            projector.delete(id, &mut *transaction).await.unwrap();

            let events = aggregate.event_store().by_aggregate_id(id).await.unwrap();

            for event in events {
                projector
                    .project(&event, &mut transaction)
                    .await
                    .expect("Failed to project event");
            }
        }

        transaction.commit().await.unwrap();
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

    let projectors: Vec<Box<dyn Projector<CounterAggregate> + Send + Sync>> = vec![Box::new(CounterProjector {})];

    let mut transaction = pool.begin().await.unwrap();

    let events: Vec<StoreEvent<CounterEvent>> = aggregate
        .event_store
        .stream_events(&mut transaction)
        .collect::<Vec<Result<StoreEvent<CounterEvent>, CounterError>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<StoreEvent<CounterEvent>>, CounterError>>()
        .expect("Failed to get all events from event_table");

    sqlx::query("TRUNCATE TABLE counters")
        .execute(&mut transaction)
        .await
        .expect("Failed to drop table");

    // Assert the counter doesn't exist
    let res = Counter::by_id(count_id, &mut transaction).await.expect("Query failed");
    assert!(res.is_none());

    rebuild_all_at_once(events, projectors.as_slice(), &mut transaction).await;

    transaction.commit().await.unwrap();

    // Assert the counter has been rebuilt
    let res = Counter::by_id(count_id, &pool)
        .await
        .expect("Query failed")
        .expect("counter not found");

    assert!(res.counter_id == count_id && res.count == 3);
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Debug)]
pub struct Event {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
}

impl<E: serde::de::DeserializeOwned> TryInto<StoreEvent<E>> for Event {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<StoreEvent<E>, Self::Error> {
        Ok(StoreEvent {
            id: self.id,
            aggregate_id: self.aggregate_id,
            payload: serde_json::from_value::<E>(self.payload)?,
            occurred_on: self.occurred_on,
            sequence_number: self.sequence_number,
        })
    }
}
