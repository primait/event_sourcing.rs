use serde::{Deserialize, Serialize};

use esrs::Aggregate;
pub use event_handler::*;
pub use view::*;

mod event_handler;
mod view;

#[derive(Clone)]
pub struct BasicAggregate;

impl Aggregate for BasicAggregate {
    const NAME: &'static str = "basic";
    type State = ();
    type Command = BasicCommand;
    type Event = BasicEvent;
    type Error = BasicError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        if command.content.is_empty() {
            Err(BasicError::EmptyContent)
        } else {
            Ok(vec![BasicEvent {
                content: command.content,
            }])
        }
    }

    fn apply_event(_state: Self::State, _payload: Self::Event) -> Self::State {}
}

pub struct BasicCommand {
    pub content: String,
}

#[derive(Serialize, Deserialize, PartialEq, Clone)]
pub struct BasicEvent {
    pub content: String,
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for BasicEvent {}

#[allow(dead_code)]
#[derive(Debug, thiserror::Error)]
pub enum BasicError {
    #[error("Empty content")]
    EmptyContent,
    #[error("Custom error: {}", .0)]
    Custom(String),
}
