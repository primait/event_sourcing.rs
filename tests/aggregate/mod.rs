use esrs::Aggregate;
pub use event_handler::*;
pub use structs::*;
#[cfg(feature = "postgres")]
pub use transactional_event_handler::*;

mod event_handler;
mod structs;
#[cfg(feature = "postgres")]
mod transactional_event_handler;

pub struct TestAggregate;

#[derive(Clone)]
pub struct TestAggregateState {
    pub count: i32,
}

impl Default for TestAggregateState {
    fn default() -> Self {
        Self { count: 1 }
    }
}

impl Aggregate for TestAggregate {
    const NAME: &'static str = "test";
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
