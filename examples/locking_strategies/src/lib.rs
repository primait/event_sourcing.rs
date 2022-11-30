use esrs::AggregateManager;
use uuid::Uuid;

use delete_aggregate::aggregate::CounterAggregate;
use delete_aggregate::structs::CounterCommand;
use delete_aggregate::structs::CounterError;

/// Increment the value behind this `aggregate_id` as soon as atomical access can be obtained.
/// The lock can be obtained even if there are current optimistic accesses! Avoid mixing the two strategies when writing.
pub async fn increment_atomically(aggregate: CounterAggregate, aggregate_id: Uuid) -> Result<(), CounterError> {
    let aggregate_state = aggregate.lock_and_load(aggregate_id).await?.unwrap_or_default();
    aggregate
        .handle_command(aggregate_state, CounterCommand::Increment)
        .await?;
    Ok(())
}

/// Increment the value behind this `aggregate_id` with an optimistic locking strategy.
/// Optimistic access ignores any current active lock! Avoid mixing the two strategies when writing.
pub async fn increment_optimistically(aggregate: CounterAggregate, aggregate_id: Uuid) -> Result<(), CounterError> {
    // Every optimistic access can take place concurrently...
    let aggregate_state = aggregate.load(aggregate_id).await?.unwrap_or_default();
    // ...and events are persisted in non-deterministic order.
    // This could raise optimistic locking errors, that must be handled downstream.
    aggregate
        .handle_command(aggregate_state, CounterCommand::Increment)
        .await?;
    Ok(())
}

/// Load the aggregate state for read-only purposes, preventing others (that use locking) from modifying it.
/// Avoid using atomic reads if writes are optimistic, as the state would be modified anyway!
/// If writes are atomic, it is perfectly fine to use a mixture of atomic and optimistic reads.
pub async fn with_atomic_read(aggregate: CounterAggregate, aggregate_id: Uuid) -> Result<(), CounterError> {
    let aggregate_state = aggregate.lock_and_load(aggregate_id).await?.unwrap_or_default();
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
pub async fn with_optimistic_read(aggregate: CounterAggregate, aggregate_id: Uuid) -> Result<(), CounterError> {
    // Read the state now, ignoring any explicit locking...
    let aggregate_state = aggregate.load(aggregate_id).await?.unwrap_or_default();
    // ...but nothing prevents something else from updating the data in the store in the meanwhile,
    // so the `aggregate_state` here might be already outdated at this point.
    println!(
        "Doing something with the state just read: {}",
        aggregate_state.next_sequence_number()
    );
    Ok(())
}
