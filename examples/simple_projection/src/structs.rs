use serde::{Deserialize, Serialize};
use thiserror::Error;

// A simple error enum for event processing errors
#[derive(Debug, Error)]
pub enum CounterError {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}

// The events to be projected
#[derive(Serialize, Deserialize, Debug)]
pub enum CounterEvent {
    Incremented,
    Decremented,
}

// The commands received by the application, which will produce the events
pub enum CounterCommand {
    Increment,
    Decrement,
}
