use crate::aggregate::{Aggregate, AggregateManager, AggregateState};
use crate::esrs::postgres::PgStore;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

#[sqlx::test]
fn handle_command_test(pool: Pool<Postgres>) {
    let aggregate: TestAggregate = TestAggregate::new(&pool).await;
    let aggregate_state: AggregateState<TestAggregateState> = AggregateState::new(Uuid::new_v4());

    let aggregate_state: AggregateState<TestAggregateState> =
        aggregate.handle(aggregate_state, TestCommand::Single).await.unwrap();
    assert_eq!(aggregate_state.inner().count, 2);
    assert_eq!(aggregate_state.sequence_number, 1);

    let aggregate_state: AggregateState<TestAggregateState> =
        aggregate.handle(aggregate_state, TestCommand::Single).await.unwrap();
    assert_eq!(aggregate_state.inner().count, 3);
    assert_eq!(aggregate_state.sequence_number, 2);

    let aggregate_state: AggregateState<TestAggregateState> =
        aggregate.handle(aggregate_state, TestCommand::Multi).await.unwrap();
    assert_eq!(aggregate_state.inner().count, 5);
    assert_eq!(aggregate_state.sequence_number, 4);
}

#[sqlx::test]
fn load_aggregate_state_test(pool: Pool<Postgres>) {
    let aggregate: TestAggregate = TestAggregate::new(&pool).await;
    let initial_aggregate_state: AggregateState<TestAggregateState> = AggregateState::new(Uuid::new_v4());

    let aggregate_state: AggregateState<TestAggregateState> = aggregate
        .handle(initial_aggregate_state.clone(), TestCommand::Multi)
        .await
        .unwrap();

    assert_eq!(initial_aggregate_state.id(), aggregate_state.id());
    assert_eq!(
        initial_aggregate_state.sequence_number + 2,
        aggregate_state.sequence_number
    );
    assert_eq!(initial_aggregate_state.inner.count + 2, aggregate_state.inner.count);

    let loaded_aggregate_state: AggregateState<TestAggregateState> =
        aggregate.load(*initial_aggregate_state.id()).await.unwrap();

    assert_eq!(aggregate_state.id, loaded_aggregate_state.id);
    assert_eq!(aggregate_state.sequence_number, loaded_aggregate_state.sequence_number);
    assert_eq!(aggregate_state.inner.count, loaded_aggregate_state.inner.count);
}

struct TestAggregate {
    event_store: PgStore<Self>,
}

impl TestAggregate {
    async fn new(pool: &Pool<Postgres>) -> Self {
        Self {
            event_store: PgStore::new(pool, vec![], vec![]).setup().await.unwrap(),
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

    fn name() -> &'static str
    where
        Self: Sized,
    {
        "test"
    }

    fn handle_command(
        _state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
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

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}

enum TestCommand {
    Single,
    Multi,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct TestEvent {
    add: i32,
}

#[derive(Debug)]
pub struct TestError;

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
