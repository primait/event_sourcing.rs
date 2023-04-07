use std::fmt::{Display, Formatter};

use chrono::Utc;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use crate::esrs::event::Event;
use crate::esrs::SequenceNumber;
use crate::postgres::PgStore;
use crate::{Aggregate, AggregateManager, AggregateState, EventStore};

#[sqlx::test]
async fn handle_command_test(pool: Pool<Postgres>) {
    let aggregate: TestAggregate = TestAggregate::new(&pool).await;
    let aggregate_state: AggregateState<TestAggregateState> = AggregateState::new();
    let aggregate_id = *aggregate_state.id();

    aggregate
        .handle_command(aggregate_state, TestCommand::Single)
        .await
        .unwrap();

    let aggregate_state = aggregate.lock_and_load(aggregate_id).await.unwrap().unwrap();
    assert_eq!(aggregate_state.inner().count, 2);
    assert_eq!(aggregate_state.sequence_number(), &1);

    aggregate
        .handle_command(aggregate_state, TestCommand::Single)
        .await
        .unwrap();

    let aggregate_state = aggregate.lock_and_load(aggregate_id).await.unwrap().unwrap();
    assert_eq!(aggregate_state.inner().count, 3);
    assert_eq!(aggregate_state.sequence_number(), &2);

    aggregate
        .handle_command(aggregate_state, TestCommand::Multi)
        .await
        .unwrap();

    let aggregate_state = aggregate.lock_and_load(aggregate_id).await.unwrap().unwrap();
    assert_eq!(aggregate_state.inner().count, 5);
    assert_eq!(aggregate_state.sequence_number(), &4);
}

#[sqlx::test]
async fn load_aggregate_state_test(pool: Pool<Postgres>) {
    let aggregate: TestAggregate = TestAggregate::new(&pool).await;
    let initial_aggregate_state: AggregateState<TestAggregateState> = AggregateState::new();

    let initial_id = *initial_aggregate_state.id();
    let initial_sequence_number = *initial_aggregate_state.sequence_number();
    let initial_count = initial_aggregate_state.inner().count;

    aggregate
        .handle_command(initial_aggregate_state, TestCommand::Multi)
        .await
        .unwrap();

    let aggregate_state = aggregate.lock_and_load(initial_id).await.unwrap().unwrap();
    assert_eq!(&initial_id, aggregate_state.id());
    assert_eq!(initial_sequence_number + 2, *aggregate_state.sequence_number());
    assert_eq!(initial_count + 2, aggregate_state.inner().count);
}

#[cfg(feature = "upcasting")]
#[sqlx::test]
async fn upcast_ok(pool: Pool<Postgres>) {
    let id = Uuid::new_v4();
    let aggregate_id = Uuid::new_v4();
    let payload: Value = serde_json::from_str("{\"val\": \"3\"}").unwrap();

    let aggregate = TestAggregate::new(&pool).await;

    insert_event(id, aggregate_id, payload, &pool).await;

    let events = aggregate.event_store.by_aggregate_id(aggregate_id).await.unwrap();
    assert!(!events.is_empty());
    let upcasted_event = events.first().unwrap();
    assert_eq!(upcasted_event.id, id);

    assert_eq!(upcasted_event.payload.add, 3);
}

#[cfg(feature = "upcasting")]
#[sqlx::test]
async fn upcast_fail_parse_value(pool: Pool<Postgres>) {
    let id = Uuid::new_v4();
    let aggregate_id = Uuid::new_v4();
    let payload: Value = serde_json::from_str("{\"val\": \"a\"}").unwrap();

    let aggregate = TestAggregate::new(&pool).await;

    insert_event(id, aggregate_id, payload, &pool).await;

    let err = aggregate.event_store.by_aggregate_id(aggregate_id).await.unwrap_err();
    assert!(matches!(err, TestError::Json(_)));
}

#[cfg(feature = "upcasting")]
#[sqlx::test]
async fn upcast_fail_not_existing_field(pool: Pool<Postgres>) {
    let id = Uuid::new_v4();
    let aggregate_id = Uuid::new_v4();
    let payload: Value = serde_json::from_str("{\"no_val\": \"3\"}").unwrap();

    let aggregate = TestAggregate::new(&pool).await;

    insert_event(id, aggregate_id, payload, &pool).await;

    let err = aggregate.event_store.by_aggregate_id(aggregate_id).await.unwrap_err();
    assert!(matches!(err, TestError::Json(_)));
}

async fn insert_event(id: Uuid, aggregate_id: Uuid, payload: Value, pool: &Pool<Postgres>) {
    let query: String = format!(
        include_str!("../postgres/statements/insert.sql"),
        format!("{}_events", TestAggregate::name())
    );
    // Inserting old version of the event
    let _ = sqlx::query(query.as_str())
        .bind(id)
        .bind(aggregate_id)
        .bind(Json(&payload))
        .bind(Utc::now())
        .bind(SequenceNumber::default())
        .execute(pool)
        .await
        .expect("Failed to insert event");
}

struct TestAggregate {
    event_store: PgStore<Self>,
}

impl TestAggregate {
    async fn new(pool: &Pool<Postgres>) -> Self {
        Self {
            event_store: PgStore::new(pool.clone()).setup().await.unwrap(),
        }
    }
}

#[derive(Clone)]
pub struct TestAggregateState {
    count: u32,
}

impl Default for TestAggregateState {
    fn default() -> Self {
        Self { count: 1 }
    }
}

impl Aggregate for TestAggregate {
    type State = TestAggregateState;
    type Command = TestCommand;
    type Event = TestEvent;
    type Error = TestError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            TestCommand::Single => Ok(vec![TestEvent { add: 1 }]),
            TestCommand::Multi => Ok(vec![TestEvent { add: 1 }, TestEvent { add: 1 }]),
        }
    }

    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State {
        Self::State {
            count: state.count + payload.add,
        }
    }
}

impl AggregateManager for TestAggregate {
    type EventStore = PgStore<Self>;

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "test"
    }

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

enum TestCommand {
    Single,
    Multi,
}

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug)]
struct TestEvent {
    add: u32,
}

impl Event for TestEvent {}

#[cfg(feature = "upcasting")]
impl crate::esrs::event::Upcaster for TestEvent {
    fn upcast(value: Value) -> Result<Self, serde_json::Error> {
        use serde::de::Error;
        use std::str::FromStr;

        // First of all try to deserialize the event using current version
        if let Ok(event) = serde_json::from_value::<Self>(value.clone()) {
            return Ok(event);
        }

        // Then try to get it from older event.
        // For this i assume there's another event in the event store shaped as
        // struct TestEvent {
        //     val: String,
        // }
        match value {
            Value::Object(fields) => match fields.get("val") {
                None => Err(serde_json::Error::custom("TestEvent not serializable")),
                Some(value) => match value {
                    Value::String(str) => {
                        let value: u32 =
                            u32::from_str(&str).map_err(|_| serde_json::Error::custom("TestEvent not serializable"))?;
                        Ok(TestEvent { add: value })
                    }
                    _ => Err(serde_json::Error::custom("TestEvent not serializable")),
                },
            },
            _ => Err(serde_json::Error::custom("TestEvent not serializable")),
        }

        // Note: another approach for this is having older version of the event and try to deserialize
        // it with older versions cascading.
        // Otherwise it's possible to keep a version number inside of the event and try to deserialize
        // it based on that.
    }
}

#[derive(Debug)]
pub enum TestError {
    Sqlx(sqlx::Error),
    Json(serde_json::Error),
}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "test error")
    }
}

impl std::error::Error for TestError {}

impl From<sqlx::Error> for TestError {
    fn from(v: sqlx::Error) -> Self {
        TestError::Sqlx(v)
    }
}

impl From<serde_json::Error> for TestError {
    fn from(v: serde_json::Error) -> Self {
        TestError::Json(v)
    }
}
