use std::sync::{Arc, Mutex, MutexGuard};

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::store::postgres::{PgStore, PgStoreBuilder, PgStoreError};
use esrs::store::{EventStore, StoreEvent};
use esrs::{Aggregate, AggregateState};

use crate::aggregate::{TestAggregate, TestAggregateState, TestEvent, TestEventHandler, TestTransactionalEventHandler};

#[sqlx::test]
async fn setup_database_test(pool: Pool<Postgres>) {
    let table_name: String = format!("{}_events", TestAggregate::NAME);

    let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
        .bind(table_name.as_str())
        .fetch_all(&pool)
        .await
        .unwrap();

    assert!(rows.is_empty());

    let _: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone())
        .try_build()
        .await
        .expect("Failed to create PgStore");

    let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
        .bind(table_name.as_str())
        .fetch_all(&pool)
        .await
        .unwrap();

    assert!(!rows.is_empty());

    let rows = sqlx::query("SELECT indexname FROM pg_indexes WHERE tablename = $1")
        .bind(table_name.as_str())
        .fetch_all(&pool)
        .await
        .unwrap();

    // primary key, aggregate_id, aggregate_id-sequence_number
    assert_eq!(rows.len(), 3);
}

#[sqlx::test]
async fn by_aggregate_id_insert_and_delete_by_aggregate_id_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let aggregate_id: Uuid = Uuid::new_v4();

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert!(store_events.is_empty());

    let mut aggregate_state: AggregateState<TestAggregateState> = AggregateState::with_id(aggregate_id);

    let store_events: Vec<StoreEvent<TestEvent>> = store
        .persist(&mut aggregate_state, vec![TestEvent { add: 1 }])
        .await
        .unwrap();

    assert_eq!(store_events.len(), 1);
    let store_event = store_events.first().unwrap();
    assert_eq!(store_event.aggregate_id, aggregate_id);
    assert_eq!(store_event.payload.add, 1);
    // Sequence numbers starts from 1 and not from 0.
    assert_eq!(store_event.sequence_number, 1);

    // This starts again with sequence_number = 0 being that we didn't load it or we didn't apply the
    // events on that state.
    let mut aggregate_state: AggregateState<TestAggregateState> = AggregateState::with_id(aggregate_id);

    let store_events: Result<Vec<StoreEvent<TestEvent>>, PgStoreError> =
        store.persist(&mut aggregate_state, vec![TestEvent { add: 1 }]).await;

    // Violation of aggregate_id - sequence_number unique constraint
    assert!(store_events.is_err());

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 1);

    store.delete(aggregate_id).await.unwrap();

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 0);

    // Is idempotent
    store.delete(aggregate_id).await.unwrap();

    assert!(store_events.is_empty());
}

#[sqlx::test]
async fn persist_single_event_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let mut aggregate_state = AggregateState::new();
    let aggregate_id = *aggregate_state.id();

    let store_event: Vec<StoreEvent<TestEvent>> =
        EventStore::persist(&store, &mut aggregate_state, vec![TestEvent { add: 1 }])
            .await
            .unwrap();

    assert_eq!(store_event[0].aggregate_id, aggregate_id);
    assert_eq!(store_event[0].payload.add, 1);
    assert_eq!(store_event[0].sequence_number, 1);

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 1);
}

#[sqlx::test]
async fn persist_multiple_events_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let test_event_1: TestEvent = TestEvent { add: 1 };
    let test_event_2: TestEvent = TestEvent { add: 2 };
    let mut aggregate_state = AggregateState::new();
    let aggregate_id = *aggregate_state.id();

    let store_event: Vec<StoreEvent<TestEvent>> = EventStore::persist(
        &store,
        &mut aggregate_state,
        vec![test_event_1.clone(), test_event_2.clone()],
    )
    .await
    .unwrap();

    assert_eq!(store_event.len(), 2);
    assert_eq!(store_event[0].aggregate_id, aggregate_id);
    assert_eq!(store_event[0].payload.add, test_event_1.add);
    assert_eq!(store_event[0].sequence_number, 1);
    assert_eq!(store_event[1].aggregate_id, aggregate_id);
    assert_eq!(store_event[1].payload.add, test_event_2.add);
    assert_eq!(store_event[1].sequence_number, 2);

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 2);
}

#[sqlx::test]
async fn event_handling_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone())
        .add_transactional_event_handler(TestTransactionalEventHandler)
        .try_build()
        .await
        .unwrap();

    create_test_projection_table(&pool).await;

    let mut aggregate_state = AggregateState::new();

    let _store_event: Vec<StoreEvent<TestEvent>> =
        EventStore::persist(&store, &mut aggregate_state, vec![TestEvent { add: 1 }])
            .await
            .unwrap();

    let projection_rows = sqlx::query_as::<_, ProjectionRow>("SELECT * FROM test_projection")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(projection_rows.len(), 1);
    assert_eq!(projection_rows[0].id, *aggregate_state.id());
    assert_eq!(projection_rows[0].total, 1);
}

#[sqlx::test]
async fn delete_store_events_and_handle_events_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone())
        .add_transactional_event_handler(TestTransactionalEventHandler)
        .try_build()
        .await
        .unwrap();

    create_test_projection_table(&pool).await;

    let mut aggregate_state = AggregateState::new();
    let aggregate_id = *aggregate_state.id();

    let _store_event: Vec<StoreEvent<TestEvent>> =
        EventStore::persist(&store, &mut aggregate_state, vec![TestEvent { add: 1 }])
            .await
            .unwrap();

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 1);

    let projection_rows = sqlx::query_as::<_, ProjectionRow>("SELECT * FROM test_projection")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(projection_rows.len(), 1);
    assert_eq!(projection_rows[0].id, aggregate_id);
    assert_eq!(projection_rows[0].total, 1);

    store.delete(aggregate_id).await.unwrap();

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 0);

    let projection_rows = sqlx::query_as::<_, ProjectionRow>("SELECT * FROM test_projection")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert!(projection_rows.is_empty());
}

#[sqlx::test]
async fn event_handler_test(pool: Pool<Postgres>) {
    let total: Arc<Mutex<i32>> = Arc::new(Mutex::new(100));
    let event_handler: TestEventHandler = TestEventHandler { total: total.clone() };

    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(event_handler)
        .try_build()
        .await
        .unwrap();

    let mut aggregate_state = AggregateState::new();

    let _store_event: Vec<StoreEvent<TestEvent>> =
        EventStore::persist(&store, &mut aggregate_state, vec![TestEvent { add: 1 }])
            .await
            .unwrap();

    let guard: MutexGuard<i32> = total.lock().unwrap();
    assert_eq!(*guard, 101);
}

async fn create_test_projection_table(pool: &Pool<Postgres>) {
    let _ = sqlx::query("DROP TABLE IF EXISTS test_projection")
        .execute(pool)
        .await
        .unwrap();

    let _ = sqlx::query("CREATE TABLE test_projection (id uuid PRIMARY KEY NOT NULL, total INTEGER)")
        .execute(pool)
        .await
        .unwrap();
}

#[derive(sqlx::FromRow, Debug)]
pub struct ProjectionRow {
    pub id: Uuid,
    pub total: i32,
}
