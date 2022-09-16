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

// The events produced by the aggregates. These are empty structs as their contents don't
// matter for this example
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EventA {
    Inner,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum EventB {
    Inner,
}

// The commands received by the application, which will produce the events
// These are empty structs since their actual contents dont matter for this example
pub enum CommandA {
    Inner,
}

pub enum CommandB {
    Inner,
}
