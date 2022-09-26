use serde::{Deserialize, Serialize};
use thiserror::Error;

// A simple error enum for event processing errors
#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("Err {0}")]
    Domain(String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}

// The events to be processed. On receipt of a new log message, the aggregate stores a Received event.
// If the message contained within can be logged, it is, and then a Succeeded event is produced, otherwise
// a Failed event is produced.
#[derive(Serialize, Deserialize, Debug)]
pub enum LoggingEvent {
    Received(String),
    Succeeded,
    Failed,
}

// The aggregate receives commands to log a message
pub enum LoggingCommand {
    TryLog(String),
    Succeed,
    Fail,
}
