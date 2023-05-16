use sqlx::{Pool, Postgres};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState};

use crate::aggregate::{TestAggregate, TestAggregateState, TestCommand};

#[sqlx::test]
async fn handle_command_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool).try_build().await.unwrap();
    let manager: AggregateManager<TestAggregate> = AggregateManager::new(Box::new(store));

    let aggregate_state: AggregateState<TestAggregateState> = AggregateState::new();
    let aggregate_id = *aggregate_state.id();

    manager
        .handle_command(aggregate_state, TestCommand::Single)
        .await
        .unwrap();

    let aggregate_state = manager.load(aggregate_id).await.unwrap().unwrap();
    assert_eq!(aggregate_state.inner().count, 2);
    assert_eq!(aggregate_state.sequence_number(), &1);

    manager
        .handle_command(aggregate_state, TestCommand::Single)
        .await
        .unwrap();

    let aggregate_state = manager.load(aggregate_id).await.unwrap().unwrap();
    assert_eq!(aggregate_state.inner().count, 3);
    assert_eq!(aggregate_state.sequence_number(), &2);

    manager
        .handle_command(aggregate_state, TestCommand::Multi)
        .await
        .unwrap();

    let aggregate_state = manager.load(aggregate_id).await.unwrap().unwrap();
    assert_eq!(aggregate_state.inner().count, 5);
    assert_eq!(aggregate_state.sequence_number(), &4);
}

#[sqlx::test]
async fn load_aggregate_state_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool).try_build().await.unwrap();
    let manager: AggregateManager<TestAggregate> = AggregateManager::new(Box::new(store));

    let initial_aggregate_state: AggregateState<TestAggregateState> = AggregateState::new();

    let initial_id = *initial_aggregate_state.id();
    let initial_sequence_number = *initial_aggregate_state.sequence_number();
    let initial_count = initial_aggregate_state.inner().count;

    manager
        .handle_command(initial_aggregate_state, TestCommand::Multi)
        .await
        .unwrap();

    let aggregate_state = manager.load(initial_id).await.unwrap().unwrap();
    assert_eq!(&initial_id, aggregate_state.id());
    assert_eq!(initial_sequence_number + 2, *aggregate_state.sequence_number());
    assert_eq!(initial_count + 2, aggregate_state.inner().count);
}

#[sqlx::test]
async fn lock_and_load_aggregate_state_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool).try_build().await.unwrap();
    let manager: AggregateManager<TestAggregate> = AggregateManager::new(Box::new(store));

    let initial_aggregate_state: AggregateState<TestAggregateState> = AggregateState::new();

    let initial_id = *initial_aggregate_state.id();
    let initial_sequence_number = *initial_aggregate_state.sequence_number();
    let initial_count = initial_aggregate_state.inner().count;

    manager
        .handle_command(initial_aggregate_state, TestCommand::Multi)
        .await
        .unwrap();

    let aggregate_state_1 = manager.lock_and_load(initial_id).await.unwrap().unwrap();
    assert_eq!(&initial_id, aggregate_state_1.id());
    assert_eq!(initial_sequence_number + 2, *aggregate_state_1.sequence_number());
    assert_eq!(initial_count + 2, aggregate_state_1.inner().count);

    let thread_executed: Arc<Mutex<bool>> = Arc::new(Mutex::new(false));
    let cloned = thread_executed.clone();

    // Just an hack to check that the lock_and_load below is still pending while this thread completes.
    tokio::spawn(async move {
        std::thread::sleep(Duration::from_millis(100));
        let mut guard = cloned.lock().unwrap();
        *guard = true;
        drop(aggregate_state_1);
    });

    // Deadlock here
    let aggregate_state_2 = manager.lock_and_load(initial_id).await.unwrap().unwrap();
    assert!(*thread_executed.lock().unwrap());
    assert_eq!(&initial_id, aggregate_state_2.id());
    assert_eq!(initial_sequence_number + 2, *aggregate_state_2.sequence_number());
    assert_eq!(initial_count + 2, aggregate_state_2.inner().count);
}

#[sqlx::test]
async fn lock_and_load_aggregate_state_test_2(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool).try_build().await.unwrap();
    let manager: AggregateManager<TestAggregate> = AggregateManager::new(Box::new(store));

    let initial_aggregate_state: AggregateState<TestAggregateState> = AggregateState::new();

    let initial_id = *initial_aggregate_state.id();
    let initial_sequence_number = *initial_aggregate_state.sequence_number();
    let initial_count = initial_aggregate_state.inner().count;

    manager
        .handle_command(initial_aggregate_state, TestCommand::Multi)
        .await
        .unwrap();

    let aggregate_state_1 = manager.lock_and_load(initial_id).await.unwrap().unwrap();
    assert_eq!(&initial_id, aggregate_state_1.id());
    assert_eq!(initial_sequence_number + 2, *aggregate_state_1.sequence_number());
    assert_eq!(initial_count + 2, aggregate_state_1.inner().count);

    // No deadlock here when using `load`
    let aggregate_state_2 = manager.load(initial_id).await.unwrap().unwrap();
    assert_eq!(&initial_id, aggregate_state_2.id());
    assert_eq!(initial_sequence_number + 2, *aggregate_state_2.sequence_number());
    assert_eq!(initial_count + 2, aggregate_state_2.inner().count);
}

#[sqlx::test]
async fn delete_aggregate_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool).try_build().await.unwrap();
    let manager: AggregateManager<TestAggregate> = AggregateManager::new(Box::new(store));

    let initial_aggregate_state: AggregateState<TestAggregateState> = AggregateState::new();

    let initial_id = *initial_aggregate_state.id();
    let initial_sequence_number = *initial_aggregate_state.sequence_number();
    let initial_count = initial_aggregate_state.inner().count;

    manager
        .handle_command(initial_aggregate_state, TestCommand::Multi)
        .await
        .unwrap();

    let aggregate_state = manager.load(initial_id).await.unwrap().unwrap();
    assert_eq!(&initial_id, aggregate_state.id());
    assert_eq!(initial_sequence_number + 2, *aggregate_state.sequence_number());
    assert_eq!(initial_count + 2, aggregate_state.inner().count);

    let deletion_result = manager.delete(initial_id).await;
    assert!(deletion_result.is_ok());

    let aggregate_state = manager.load(initial_id).await.unwrap();
    assert!(aggregate_state.is_none());
}
