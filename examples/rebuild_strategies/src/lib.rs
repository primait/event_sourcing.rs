use futures_util::stream::StreamExt;
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use aggregate_merging::aggregates::{AggregateA, AggregateB};
use aggregate_merging::structs::{CounterError, EventA, EventB};
use esrs::postgres::{PgStore, Projector};
use esrs::{AggregateManager, EventStore, StoreEvent};

/// A simple example demonstrating rebuilding a single projection table from an aggregate.
pub async fn rebuild_single_projection_all_at_once(pool: Pool<Postgres>) {
    let aggregate: AggregateA = AggregateA::new(&pool).await.unwrap();

    // Put here all the projector you want to rebuild, but remember to add a truncate table statement
    // for every table the projectors inside this vec insist on.
    let projectors: Vec<Box<dyn Projector<AggregateA>>> = vec![];

    // Start the transaction
    let mut transaction: Transaction<Postgres> = pool.begin().await.unwrap();

    // Get all events from the event_store
    let events: Vec<StoreEvent<EventA>> = aggregate
        .event_store
        .stream_events(&mut *transaction)
        .collect::<Vec<Result<StoreEvent<EventA>, CounterError>>>()
        .await
        .into_iter()
        .collect::<Result<Vec<StoreEvent<EventA>>, CounterError>>()
        .expect("Failed to get all events from event_table");

    // From within the transaction truncate the projection table you are going to rebuild
    sqlx::query("TRUNCATE TABLE counters")
        .execute(&mut *transaction)
        .await
        .expect("Failed to drop table");

    // Then fully rebuild the projection table
    for event in events {
        for projector in projectors.iter() {
            projector
                .project(&event, &mut transaction)
                .await
                .expect("Failed to project event");
        }
    }

    // And finally commit your transaction
    transaction.commit().await.unwrap();
}

/// An alternative approach to rebuilding that rebuilds the projected table for a given projection one
/// aggregate ID at a time, rather than committing the entire table all at once
pub async fn rebuild_single_projection_per_aggregate_id(pool: Pool<Postgres>) {
    let aggregate: AggregateA = AggregateA::new(&pool).await.unwrap();

    // Put here all the projector for all the projections you want to rebuild
    let projectors: Vec<Box<dyn Projector<AggregateA>>> = vec![];

    // Get all unique aggregate_ids from event_store table. This should be a sqlx::query statement.
    let aggregate_ids: Vec<Uuid> = vec![Uuid::new_v4()];

    // For every aggregate_id..
    for aggregate_id in aggregate_ids {
        // .. open a transaction..
        let mut transaction: Transaction<Postgres> = pool.begin().await.unwrap();

        // .. and for every projector..
        for projector in projectors.iter() {
            // .. delete all the records in the projection that has that aggregate_id as key. In order
            // to achieve this remember to override default `delete` implementation in the projector.
            projector.delete(aggregate_id, &mut transaction).await.unwrap();

            // Then queries for all the events in the event store table..
            let events = aggregate.event_store().by_aggregate_id(aggregate_id).await.unwrap();

            // .. and rebuild all those events.
            for event in events {
                projector
                    .project(&event, &mut transaction)
                    .await
                    .expect("Failed to project event");
            }
        }

        // And finally commit your transaction
        transaction.commit().await.unwrap();
    }
}

/// A simple example demonstrating rebuilding a shared projection streaming on two different event
/// stores
pub async fn rebuild_shared_projection_streaming(pool: Pool<Postgres>) {
    // Build both the stores
    let store_a: PgStore<AggregateA> = PgStore::new(pool.clone());
    let store_b: PgStore<AggregateB> = PgStore::new(pool.clone());

    // Put here all the projector from AggregateA you want to rebuild, but remember to add a truncate
    // table statement for every table the projectors inside this vec insist on.
    let projectors_a: Vec<Box<dyn Projector<AggregateA>>> = vec![];
    // Put here all the projector from AggregateB you want to rebuild, but remember to add a truncate
    // table statement for every table the projectors inside this vec insist on.
    let projectors_b: Vec<Box<dyn Projector<AggregateB>>> = vec![];

    // Get two streams from both the tables
    let mut events_a = store_a.stream_events(&pool);
    let mut events_b = store_b.stream_events(&pool);

    // Fetch first element of both the tables
    let mut event_a_opt: Option<Result<StoreEvent<EventA>, CounterError>> = events_a.next().await;
    let mut event_b_opt: Option<Result<StoreEvent<EventB>, CounterError>> = events_b.next().await;

    // At this point is possible to open a transaction
    let mut transaction: Transaction<Postgres> = pool.begin().await.unwrap();

    // Truncate the shared projection table.
    let _ = sqlx::query("TRUNCATE TABLE counters")
        .execute(&mut *transaction)
        .await
        .unwrap();

    loop {
        let a_opt: Option<&StoreEvent<EventA>> = event_a_opt.as_ref().map(|v| v.as_ref().unwrap());
        let b_opt: Option<&StoreEvent<EventB>> = event_b_opt.as_ref().map(|v| v.as_ref().unwrap());

        match (a_opt, b_opt) {
            // If both the streams returned a value we check what's the oldest. If the oldest is a
            // we proceed to run the projectors from AggregateA.
            (Some(a), Some(b)) if a.occurred_on <= b.occurred_on => {
                for projector in projectors_a.iter() {
                    projector.project(a, &mut transaction).await.unwrap();
                }

                // Get next value from AggregateA events stream
                event_a_opt = events_a.next().await;
            }
            // If only the stream on AggregateA events contains values we proceed to run the projectors
            // from AggregateA.
            (Some(a), None) => {
                for projector in projectors_a.iter() {
                    projector.project(a, &mut transaction).await.unwrap();
                }

                // Get next value from AggregateA events stream
                event_a_opt = events_a.next().await;
            }
            // If both the streams returned a value and AggregateB event is older or if only the stream
            // on AggregateB events contains values we proceed to run the projectors from AggregateB.
            (Some(_), Some(b)) | (None, Some(b)) => {
                for projector in projectors_b.iter() {
                    projector.project(b, &mut transaction).await.unwrap();
                }

                // Get next value from AggregateB events stream
                event_b_opt = events_b.next().await;
            }
            // If both the streams are empty then we break the loop.
            (None, None) => break,
        };
    }

    // Finally commit the transaction
    transaction.commit().await.unwrap();
}
