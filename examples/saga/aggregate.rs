use serde::{Deserialize, Serialize};

use esrs::Aggregate;

use crate::common::Error;

#[derive(Clone)]
pub struct SagaAggregate;

impl Aggregate for SagaAggregate {
    const NAME: &'static str = "saga";
    type State = ();
    type Command = SagaCommand;
    type Event = SagaEvent;
    type Error = Error;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            SagaCommand::RequestMutation => Ok(vec![SagaEvent::MutationRequested]),
            SagaCommand::RegisterMutation => Ok(vec![SagaEvent::MutationRegistered]),
        }
    }

    fn apply_event(_state: Self::State, _payload: Self::Event) -> Self::State {}
}

pub enum SagaCommand {
    RequestMutation,
    RegisterMutation,
}

#[derive(Serialize, Deserialize, PartialEq)]
pub enum SagaEvent {
    MutationRequested,
    MutationRegistered,
}
