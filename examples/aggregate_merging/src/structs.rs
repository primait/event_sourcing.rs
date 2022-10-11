use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

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

// The events produced by the aggregates. The inner id acts as shared id between them
#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventA {
    Inner { shared_id: Uuid },
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum EventB {
    Inner { shared_id: Uuid },
}

// The commands received by the application, which will produce the events
// The inner id acts as shared id between them
pub enum CommandA {
    Inner { shared_id: Uuid },
}

pub enum CommandB {
    Inner { shared_id: Uuid },
}
