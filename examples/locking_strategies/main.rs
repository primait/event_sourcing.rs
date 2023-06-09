//! # Locking Strategies Example
//!
//! This example is divided into two sections, each focusing on different aspects of locking strategies:
//! Optimistic Concurrency Control (OCC) and Pessimistic Concurrency Control (PCC).
//!
//! ## Section 1: Optimistic and Pessimistic Concurrency Control
//! The first section demonstrates how to implement and utilize Optimistic Concurrency Control and
//! Pessimistic Concurrency Control in event sourcing scenarios.
//!
//! - Optimistic Concurrency Control (OCC):
//!   This part showcases the usage of optimistic locks, which allow multiple threads to read and
//!   modify data concurrently. The example illustrates how conflicts are detected during the write
//!   operation by employing a versioning mechanism.
//!
//! - Pessimistic Concurrency Control (PCC):
//!   In this section, the example demonstrates the application of pessimistic locks, which acquire
//!   locks on data before performing modifications. It showcases how these locks ensure exclusive
//!   access to data, preventing concurrent modifications and maintaining data integrity.
//!
//! ## Section 2: Showcase of Optimistic and Pessimistic Locks
//! The second section of the example provides a detailed showcase of how optimistic and pessimistic
//! locks function in practice.
//!
//! - Pessimistic Locking Showcase:
//!   This part demonstrates the behavior of a pessimistic lock where one thread remains blocked
//!   while another thread holds a lock on a specific aggregate state. The example employs guard
//!   assertions to verify that the blocked thread remains suspended until the lock is released by
//!   the other thread.
//!
//! - Optimistic Locking Showcase:
//!   This section highlights the versatility of optimistic locking, even when a pessimistic lock is
//!   already in progress. It demonstrates how optimistic locks can still be utilized effectively,
//!   allowing other operations to proceed while ensuring conflicts are detected and resolved during
//!   write operations.

use std::sync::Arc;
use std::time::Duration;

use sqlx::{Pool, Postgres};
use tokio::sync::Mutex;
use uuid::Uuid;

use esrs::manager::{AggregateManager, AggregateManagerError};
use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::store::EventStore;
use esrs::AggregateState;

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicEvent};

#[path = "../common/lib.rs"]
mod common;

type Agg = AggregateManager<PgStore<BasicAggregate>>;

/// Increment the value behind this `aggregate_id` as soon as atomical access can be obtained.
/// The lock can be obtained even if there are current optimistic accesses! Avoid mixing the two strategies when writing.
pub async fn increment_atomically(
    manager: Agg,
    aggregate_id: Uuid,
) -> Result<(), AggregateManagerError<PgStore<BasicAggregate>>> {
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
pub async fn increment_optimistically(
    manager: Agg,
    aggregate_id: Uuid,
) -> Result<(), AggregateManagerError<PgStore<BasicAggregate>>> {
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
pub async fn with_atomic_read(
    manager: Agg,
    aggregate_id: Uuid,
) -> Result<(), AggregateManagerError<PgStore<BasicAggregate>>> {
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
pub async fn with_optimistic_read(
    manager: Agg,
    aggregate_id: Uuid,
) -> Result<(), AggregateManagerError<PgStore<BasicAggregate>>> {
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

    let manager: AggregateManager<PgStore<BasicAggregate>> = AggregateManager::new(store.clone());

    // It is possible to load the aggregate state multiple times.
    let _state_1 = manager.load(aggregate_id).await.unwrap().unwrap();
    let _state_2 = manager.load(aggregate_id).await.unwrap().unwrap();

    let mut locked_state_1 = manager.lock_and_load(aggregate_id).await.unwrap().unwrap();
    // It is possible to load the aggregate state even if there's a state lock.
    let _state_3 = manager.load(aggregate_id).await.unwrap().unwrap();

    drop(locked_state_1.take_lock());

    let lock_info: Arc<Mutex<bool>> = Arc::new(Mutex::default());

    // Simulation of a multithread environment.
    let cloned = store.clone();
    let lock_info_cloned = lock_info.clone();
    let future_1 = tokio::spawn(async move {
        let manager = AggregateManager::new(cloned);

        assert!(!*lock_info_cloned.lock().await);
        let _state = manager.lock_and_load(aggregate_id).await.unwrap().unwrap();

        // Here is known that the aggregate state is locked. Updating lock_info `true`.
        let mut guard = lock_info_cloned.lock().await;
        *guard = true;
        drop(guard);

        tokio::time::sleep(Duration::from_secs(1)).await;

        // Updating lock_info being that after this statement the state will be dropped and the lock
        // released.
        let mut guard = lock_info_cloned.lock().await;
        *guard = false;
    });

    let future_2 = tokio::spawn(async move {
        // Give the first thread some time in order to be the first to acquire the lock.
        tokio::time::sleep(Duration::from_millis(500)).await;

        let manager = AggregateManager::new(store);

        // This asserts that the first thread is holding the lock on the aggregate state..
        assert!(*lock_info.lock().await);

        // This statement is now pending, waiting for the first thread lock to be released.
        let _state = manager.lock_and_load(aggregate_id).await.unwrap().unwrap();

        // ..and than that the first thread has released the lock, allowing this thread to take a
        // new lock.
        assert!(!*lock_info.lock().await);
    });

    let _ = tokio::join!(future_1, future_2);
}
