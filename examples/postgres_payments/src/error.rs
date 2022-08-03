use thiserror::Error;

#[derive(Debug, Error)]
pub enum BankAccountError {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),

    #[error("Negative amount")]
    NegativeAmount,

    #[error("Negative balance")]
    NegativeBalance,
}

#[derive(Debug, Error)]
pub enum CreditCardError {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    BankAccount(#[from] BankAccountError),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),

    #[error("Negative amount")]
    NegativeAmount,

    #[error("Ceiling limit reached")]
    CeilingLimitReached,
}
