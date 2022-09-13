use chrono::{DateTime, Utc};
use sqlx::{PgConnection, Pool, Postgres};
use std::sync::{Arc, Mutex, MutexGuard};
use uuid::Uuid;

use crate::aggregate::{Aggregate, AggregateManager, AggregateState};
use crate::esrs::postgres::tests::TestError;
use crate::esrs::postgres::PgStore;
use crate::store::postgres::{Policy, Projector};
use crate::store::{EventStore, StoreEvent};

#[sqlx::test]
fn setup_database_test(pool: Pool<Postgres>) {
    let store: PgStore<TestEvent, TestError> = PgStore::new::<TestAggregate>(&pool, vec![], vec![]);

    let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
        .bind(&store.table_name)
        .fetch_all(&pool)
        .await
        .unwrap();

    assert!(rows.is_empty());

    let store: PgStore<TestEvent, TestError> = store.setup().await.unwrap();

    let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
        .bind(&store.table_name)
        .fetch_all(&pool)
        .await
        .unwrap();

    assert!(!rows.is_empty());
}

#[sqlx::test]
fn by_aggregate_id_insert_and_delete_by_aggregate_id_test(pool: Pool<Postgres>) {
    let store: PgStore<TestEvent, TestError> = PgStore::new::<TestAggregate>(&pool, vec![], vec![])
        .setup()
        .await
        .unwrap();

    let event_internal_id: Uuid = Uuid::new_v4();
    let aggregate_id: Uuid = Uuid::new_v4();
    let occurred_on: DateTime<Utc> = Utc::now();

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert!(store_events.is_empty());

    let store_event: StoreEvent<TestEvent> = store
        .insert(aggregate_id, TestEvent { id: event_internal_id }, occurred_on, 0, &pool)
        .await
        .unwrap();

    assert_eq!(store_event.aggregate_id, aggregate_id);
    assert_eq!(store_event.payload.id, event_internal_id);
    assert_eq!(store_event.occurred_on, occurred_on);
    assert_eq!(store_event.sequence_number, 0);

    let store_event: Result<StoreEvent<TestEvent>, TestError> = store
        .insert(aggregate_id, TestEvent { id: event_internal_id }, occurred_on, 0, &pool)
        .await;

    // Violation of aggregate_id - sequence_number unique constraint
    assert!(store_event.is_err());

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 1);

    store.delete_by_aggregate_id(aggregate_id).await.unwrap();

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 0);

    // Is idempotent
    store.delete_by_aggregate_id(aggregate_id).await.unwrap();

    assert!(store_events.is_empty());
}

#[sqlx::test]
fn persist_single_event_test(pool: Pool<Postgres>) {
    let store: PgStore<TestEvent, TestError> = PgStore::new::<TestAggregate>(&pool, vec![], vec![])
        .setup()
        .await
        .unwrap();

    let event_internal_id: Uuid = Uuid::new_v4();
    let aggregate_id: Uuid = Uuid::new_v4();

    let store_event: Vec<StoreEvent<TestEvent>> = store
        .persist(aggregate_id, vec![TestEvent { id: event_internal_id }], 0)
        .await
        .unwrap();

    assert_eq!(store_event[0].aggregate_id, aggregate_id);
    assert_eq!(store_event[0].payload.id, event_internal_id);
    assert_eq!(store_event[0].sequence_number, 0);

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 1);
}

#[sqlx::test]
fn persist_multiple_events_test(pool: Pool<Postgres>) {
    let store: PgStore<TestEvent, TestError> = PgStore::new::<TestAggregate>(&pool, vec![], vec![])
        .setup()
        .await
        .unwrap();

    let test_event_1: TestEvent = TestEvent { id: Uuid::new_v4() };
    let test_event_2: TestEvent = TestEvent { id: Uuid::new_v4() };
    let aggregate_id: Uuid = Uuid::new_v4();

    let store_event: Vec<StoreEvent<TestEvent>> = store
        .persist(aggregate_id, vec![test_event_1.clone(), test_event_2.clone()], 0)
        .await
        .unwrap();

    assert_eq!(store_event.len(), 2);
    assert_eq!(store_event[0].aggregate_id, aggregate_id);
    assert_eq!(store_event[0].payload.id, test_event_1.id);
    assert_eq!(store_event[0].sequence_number, 0);
    assert_eq!(store_event[1].aggregate_id, aggregate_id);
    assert_eq!(store_event[1].payload.id, test_event_2.id);
    assert_eq!(store_event[1].sequence_number, 1);

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 2);
}

#[sqlx::test]
fn event_projection_test(pool: Pool<Postgres>) {
    let store: PgStore<TestEvent, TestError> =
        PgStore::new::<TestAggregate>(&pool, vec![Box::new(TestProjector {})], vec![])
            .setup()
            .await
            .unwrap();

    create_test_projection_table(&pool).await;

    let event_internal_id: Uuid = Uuid::new_v4();
    let aggregate_id: Uuid = Uuid::new_v4();

    let _store_event: Vec<StoreEvent<TestEvent>> = store
        .persist(aggregate_id, vec![TestEvent { id: event_internal_id }], 0)
        .await
        .unwrap();

    let projection_rows = sqlx::query_as::<_, ProjectionRow>("SELECT * FROM test_projection")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(projection_rows.len(), 1);
    assert_eq!(projection_rows[0].id, event_internal_id);
    assert_eq!(projection_rows[0].projection_id, aggregate_id);
}

#[sqlx::test]
fn delete_store_events_and_projections_test(pool: Pool<Postgres>) {
    let store: PgStore<TestEvent, TestError> =
        PgStore::new::<TestAggregate>(&pool, vec![Box::new(TestProjector {})], vec![])
            .setup()
            .await
            .unwrap();

    create_test_projection_table(&pool).await;

    let event_internal_id: Uuid = Uuid::new_v4();
    let aggregate_id: Uuid = Uuid::new_v4();

    let _store_event: Vec<StoreEvent<TestEvent>> = store
        .persist(aggregate_id, vec![TestEvent { id: event_internal_id }], 0)
        .await
        .unwrap();

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 1);

    let projection_rows = sqlx::query_as::<_, ProjectionRow>("SELECT * FROM test_projection")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert_eq!(projection_rows.len(), 1);
    assert_eq!(projection_rows[0].id, event_internal_id);
    assert_eq!(projection_rows[0].projection_id, aggregate_id);

    store.delete_by_aggregate_id(aggregate_id).await.unwrap();

    let store_events: Vec<StoreEvent<TestEvent>> = store.by_aggregate_id(aggregate_id).await.unwrap();
    assert_eq!(store_events.len(), 0);

    let projection_rows = sqlx::query_as::<_, ProjectionRow>("SELECT * FROM test_projection")
        .fetch_all(&pool)
        .await
        .unwrap();

    assert!(projection_rows.is_empty());
}

#[sqlx::test]
fn policy_test(pool: Pool<Postgres>) {
    let last_id: Arc<Mutex<Uuid>> = Arc::new(Mutex::new(Default::default()));
    let policy: Box<TestPolicy> = Box::new(TestPolicy {
        last_id: last_id.clone(),
    });

    let store: PgStore<TestEvent, TestError> = PgStore::new::<TestAggregate>(&pool, vec![], vec![policy])
        .setup()
        .await
        .unwrap();

    let event_internal_id: Uuid = Uuid::new_v4();
    let aggregate_id: Uuid = Uuid::new_v4();

    let _store_event: Vec<StoreEvent<TestEvent>> = store
        .persist(aggregate_id, vec![TestEvent { id: event_internal_id }], 0)
        .await
        .unwrap();

    let guard: MutexGuard<Uuid> = last_id.lock().unwrap();
    assert_eq!(*guard, event_internal_id)
}

#[sqlx::test]
fn close_test(pool: Pool<Postgres>) {
    assert!(!pool.is_closed());

    let store: PgStore<TestEvent, TestError> = PgStore::new::<TestAggregate>(&pool, vec![], vec![])
        .setup()
        .await
        .unwrap();

    store.close().await;

    assert!(pool.is_closed());
}

async fn create_test_projection_table(pool: &Pool<Postgres>) {
    let _ = sqlx::query("DROP TABLE IF EXISTS test_projection")
        .execute(pool)
        .await
        .unwrap();

    let _ = sqlx::query("CREATE TABLE test_projection (id uuid NOT NULL, projection_id uuid NOT NULL)")
        .execute(pool)
        .await
        .unwrap();
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct TestEvent {
    id: Uuid,
}

struct TestAggregate {
    event_store: PgStore<TestEvent, TestError>,
}

impl Aggregate for TestAggregate {
    type State = ();
    type Command = ();
    type Event = TestEvent;
    type Error = TestError;
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "test"
    }
    fn handle_command(
        _state: &AggregateState<Self::State>,
        _command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        todo!()
    }
    fn apply_event(_state: Self::State, _payload: Self::Event) -> Self::State {
        todo!()
    }
}

impl AggregateManager for TestAggregate {
    type EventStore = PgStore<TestEvent, TestError>;

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

struct TestProjector;

#[async_trait::async_trait]
impl Projector<TestEvent, TestError> for TestProjector {
    async fn project(&self, event: &StoreEvent<TestEvent>, connection: &mut PgConnection) -> Result<(), TestError> {
        Ok(
            sqlx::query("INSERT INTO test_projection (id, projection_id) VALUES ($1, $2)")
                .bind(event.payload.id)
                .bind(event.aggregate_id)
                .execute(connection)
                .await
                .map(|_| ())?,
        )
    }

    async fn delete(&self, aggregate_id: Uuid, connection: &mut PgConnection) -> Result<(), TestError> {
        Ok(sqlx::query("DELETE FROM test_projection WHERE projection_id = $1")
            .bind(aggregate_id)
            .execute(connection)
            .await
            .map(|_| ())?)
    }
}

#[derive(sqlx::FromRow)]
struct ProjectionRow {
    id: Uuid,
    projection_id: Uuid,
}

struct TestPolicy {
    last_id: Arc<Mutex<Uuid>>,
}

#[async_trait::async_trait]
impl Policy<TestEvent, TestError> for TestPolicy {
    async fn handle_event(&self, event: &StoreEvent<TestEvent>, _pool: &Pool<Postgres>) -> Result<(), TestError> {
        let mut guard = self.last_id.lock().unwrap();
        *guard = event.payload.id;
        Ok(())
    }
}
