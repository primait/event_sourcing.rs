use esrs::async_impl::{JsonError, SqlError};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    Json(#[from] JsonError),

    #[error(transparent)]
    Sql(#[from] SqlError),

    #[error("Negative amount")]
    NegativeAmount,

    #[error("Negative total amount")]
    NegativeTotalAmount,
}
