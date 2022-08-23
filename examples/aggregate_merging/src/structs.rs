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
pub struct EventA;
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct EventB;

// The event the projector accepts
#[derive(Serialize, Deserialize, Debug)]
pub enum ProjectorEvent {
    A,
    B,
}

// We implement From<> for the two event types to convert
// them into a common PorjectorEvent, so that we can share
// our projector implementation across two aggregates without
// needing to duplicate our projector code. This complicates
// some type bounds a little in aggregates.rs, for the sake
// of reduced code duplication
impl From<EventA> for ProjectorEvent {
    fn from(_: EventA) -> Self {
        ProjectorEvent::A
    }
}

impl From<EventB> for ProjectorEvent {
    fn from(_: EventB) -> Self {
        ProjectorEvent::B
    }
}

// The commands received by the application, which will produce the events
// These are empty structs since their actual contents dont matter for this example
pub struct CommandA;
pub struct CommandB;
