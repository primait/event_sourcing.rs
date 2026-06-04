use serde::{Deserialize, Serialize};

use esrs::{Aggregate, AggregateView};

use crate::common::CommonError;

#[derive(Clone)]
pub struct SagaAggregate;

impl Aggregate for SagaAggregate {
    const NAME: &'static str = "saga";
    type State = ();
    type Command = SagaCommand;
    type Event = SagaEvent;
    type Error = CommonError;

    fn handle_command(
        _state: AggregateView<&Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            SagaCommand::RequestMutation => Ok(vec![SagaEvent::MutationRequested]),
            SagaCommand::RegisterMutation => Ok(vec![SagaEvent::MutationRegistered]),
        }
    }

    fn apply_event(_state: AggregateView<Self::State>, _payload: Self::Event) -> Self::State {}
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

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for SagaEvent {}
