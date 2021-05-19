use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),

    #[error("Negative amount")]
    NegativeAmount,

    #[error("Negative total amount")]
    NegativeTotalAmount,
}
