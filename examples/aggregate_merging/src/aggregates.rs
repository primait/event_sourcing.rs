use esrs::Aggregate;

use crate::structs::{CommandA, CommandB, CounterError, EventA, EventB};

// We use a template here to make instantiating the near-identical
// AggregateA and AggregateB easier.
pub struct AggregateA;

impl Aggregate for AggregateA {
    const NAME: &'static str = "a";
    type State = ();
    type Command = CommandA;
    type Event = EventA;
    type Error = CounterError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            CommandA::Inner { shared_id } => Ok(vec![EventA::Inner { shared_id }]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }
}

pub struct AggregateB;

impl Aggregate for AggregateB {
    const NAME: &'static str = "b";
    type State = ();
    type Command = CommandB;
    type Event = EventB;
    type Error = CounterError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            CommandB::Inner { shared_id } => Ok(vec![EventB::Inner { shared_id }]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection is stateful
        state
    }
}
