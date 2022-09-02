use serde::{Deserialize, Serialize};
use thiserror::Error;

// A simple error enum for event processing errors
#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("[Err {0}]")]
    Domain(String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}

// The events to be processed
#[derive(Serialize, Deserialize, Debug)]
pub enum LoggingEvent {
    Logged(String),
}

// The commands received by the application, which will produce the events
pub enum LoggingCommand {
    Log(String),
}
