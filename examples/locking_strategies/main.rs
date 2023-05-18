use std::time::Duration;

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState, EventStore};

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicError, BasicEvent};

#[path = "../common/lib.rs"]
mod common;

type Agg = AggregateManager<BasicAggregate>;

/// Increment the value behind this `aggregate_id` as soon as atomical access can be obtained.
/// The lock can be obtained even if there are current optimistic accesses! Avoid mixing the two strategies when writing.
pub async fn increment_atomically(manager: Agg, aggregate_id: Uuid) -> Result<(), BasicError> {
    let aggregate_state = manager.lock_and_load(aggregate_id).await?.unwrap_or_default();
    manager
        .handle_command(
            aggregate_state,
            BasicCommand {
                content: "whatever".to_string(),
            },
        )
        .await?;
    Ok(())
}

/// Increment the value behind this `aggregate_id` with an optimistic locking strategy.
/// Optimistic access ignores any current active lock! Avoid mixing the two strategies when writing.
pub async fn increment_optimistically(manager: Agg, aggregate_id: Uuid) -> Result<(), BasicError> {
    // Every optimistic access can take place concurrently...
    let aggregate_state = manager.load(aggregate_id).await?.unwrap_or_default();
    // ...and events are persisted in non-deterministic order.
    // This could raise optimistic locking errors, that must be handled downstream.
    manager
        .handle_command(
            aggregate_state,
            BasicCommand {
                content: "whatever".to_string(),
            },
        )
        .await?;
    Ok(())
}

/// Load the aggregate state for read-only purposes, preventing others (that use locking) from modifying it.
/// Avoid using atomic reads if writes are optimistic, as the state would be modified anyway!
/// If writes are atomic, it is perfectly fine to use a mixture of atomic and optimistic reads.
pub async fn with_atomic_read(manager: Agg, aggregate_id: Uuid) -> Result<(), BasicError> {
    let mut aggregate_state = manager.lock_and_load(aggregate_id).await?.unwrap_or_default();
    // No one else (employing locking!) can read or modify the state just loaded here,
    // ensuring this really is the *latest* aggregate state.
    println!(
        "Doing something with the state just read: {}",
        aggregate_state.next_sequence_number()
    );
    Ok(())
}

/// Load the aggregate state for read-only purposes, optimistically assuming nothing is modifying it.
/// If writes are atomic, it is perfectly fine to use a mixture of atomic and optimistic reads.
/// Otherwise, optimistic reads are allowed: beware there are no guarantees the state loaded is actually the latest.
pub async fn with_optimistic_read(manager: Agg, aggregate_id: Uuid) -> Result<(), BasicError> {
    // Read the state now, ignoring any explicit locking...
    let mut aggregate_state = manager.load(aggregate_id).await?.unwrap_or_default();
    // ...but nothing prevents something else from updating the data in the store in the meanwhile,
    // so the `aggregate_state` here might be already outdated at this point.
    println!(
        "Doing something with the state just read: {}",
        aggregate_state.next_sequence_number()
    );
    Ok(())
}

/// Locking showcase
#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let store: PgStore<BasicAggregate> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let aggregate_id: Uuid = Uuid::new_v4();
    let mut aggregate_state: AggregateState<()> = AggregateState::with_id(aggregate_id);
    let event = BasicEvent {
        content: "insert event content".to_string(),
    };
    let _ = store.persist(&mut aggregate_state, vec![event]).await.unwrap();

    let manager: AggregateManager<BasicAggregate> = AggregateManager::new(store.clone());

    // It is possible to load the aggregate state multiple times.
    let _state_1 = manager.load(aggregate_id).await.unwrap().unwrap();
    let _state_2 = manager.load(aggregate_id).await.unwrap().unwrap();

    let mut locked_state_1 = manager.lock_and_load(aggregate_id).await.unwrap().unwrap();
    // It is possible to load the aggregate state even if there's a state lock.
    let _state_3 = manager.load(aggregate_id).await.unwrap().unwrap();

    drop(locked_state_1.take_lock());

    // Simulation of a multithread environment
    let cloned = store.clone();
    let future_1 = tokio::spawn(async move {
        let manager = AggregateManager::new(cloned);
        println!("First thread: requesting lock");
        let mut state = manager.lock_and_load(aggregate_id).await.unwrap().unwrap();
        println!("First thread: locked");
        let _ = tokio::time::sleep(Duration::from_secs(2)).await;
        println!("First thread: completed its job");
        // The lock is released
        drop(state.take_lock());
        println!("First thread: lock released");
        // Giving here the time to the second thread to acquire the lock
        tokio::time::sleep(Duration::from_millis(100)).await
    });

    let cloned = store.clone();
    let future_2 = tokio::spawn(async move {
        // Give the first thread some time in order to be the first to lock
        tokio::time::sleep(Duration::from_millis(10)).await;
        let manager = AggregateManager::new(cloned);
        println!("Second thread: requesting lock");
        let mut state = manager.lock_and_load(aggregate_id).await.unwrap().unwrap();
        println!("Second thread: locked");
        println!("Second thread: completed its job");
        // The lock is released when goes out of scope. In this case, for the sake of demonstrate this
        // we will release it manually.
        drop(state.take_lock());
        println!("Second thread: lock released");
    });

    let _ = tokio::join!(future_1, future_2);

    // Expected output:
    // > First thread: requesting lock
    // > First thread: locked
    // > Second thread: requesting lock
    // > First thread: completed its job
    // > First thread: lock released
    // > Second thread: locked
    // > Second thread: completed its job
    // > Second thread: lock released
}
