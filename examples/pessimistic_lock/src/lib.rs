use esrs::AggregateManager;
use uuid::Uuid;

use crate::aggregate::CountingAggregate;
use crate::aggregate::CountingCommand;
use crate::aggregate::CountingError;

mod aggregate;

/// Increment the value behind this `aggregate_id` as soon as atomical access can be obtained.
/// The lock can be obtained even if there are current optimistic accesses! Avoid mixing the two strategies.
pub async fn increment_atomically(aggregate: CountingAggregate, aggregate_id: Uuid) -> Result<u64, CountingError> {
    // Obtain a lock for this aggregate_id, or wait for an existing one to be released.
    let _guard = aggregate.lock(aggregate_id).await?;
    // Only one atomical access at a time can proceed further.
    let aggregate_state = aggregate.load(aggregate_id).await.unwrap_or_default();
    let new_state = aggregate
        .handle_command(aggregate_state, CountingCommand::Increment)
        .await?;
    Ok(*new_state.inner())
}

/// Increment the value behind this `aggregate_id` with an optimistic locking strategy.
/// Optimistic access ignores any current active lock! Avoid mixing the two strategies.
pub async fn increment_optimistically(aggregate: CountingAggregate, aggregate_id: Uuid) -> Result<u64, CountingError> {
    // Every optimistic access can take place concurrently...
    let aggregate_state = aggregate.load(aggregate_id).await.unwrap_or_default();
    // ...and events are persisted in non-deterministic order.
    // This could raise optimistic locking errors, that must be handled downstream.
    let new_state = aggregate
        .handle_command(aggregate_state, CountingCommand::Increment)
        .await?;
    Ok(*new_state.inner())
}
