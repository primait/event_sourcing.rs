use thiserror::Error;

pub mod a;
pub mod aggregate;
pub mod b;

// A simple error enum for event processing errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}

// The commands received by the application, which will produce the events
pub enum Command {
    Increment { u: u32 },
    Decrement { u: u32 },
}
