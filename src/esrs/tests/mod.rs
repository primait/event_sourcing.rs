use std::fmt::{Display, Formatter};

use sqlx::{Pool, Postgres};

use crate::esrs::event::Event;
use crate::postgres::PgStore;
use crate::{Aggregate, AggregateManager, AggregateState};

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
    count: i32,
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
    add: i32,
}

impl Event for TestEvent {}

#[cfg(feature = "upcasting")]
impl crate::esrs::event::Upcaster for TestEvent {
    fn upcast(value: serde_json::Value) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

#[derive(Debug)]
pub struct TestError;

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "test error")
    }
}

impl std::error::Error for TestError {}

impl From<sqlx::Error> for TestError {
    fn from(_: sqlx::Error) -> Self {
        TestError
    }
}

impl From<serde_json::Error> for TestError {
    fn from(_: serde_json::Error) -> Self {
        TestError
    }
}
