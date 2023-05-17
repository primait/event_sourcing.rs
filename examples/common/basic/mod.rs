use esrs::Aggregate;
use serde::{Deserialize, Serialize};
pub use view::*;

use crate::common::Error;

mod view;

pub const BASIC_TABLE_NAME: &str = "basic_view";

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

#[derive(Serialize, Deserialize)]
pub struct BasicEvent {
    pub content: String,
}

#[derive(Debug, Error)]
pub enum BasicError {
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error("Empty content")]
    EmptyContent,
    #[error("Custom error {0}")]
    Custom(String),
}
