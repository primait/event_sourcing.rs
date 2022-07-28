use thiserror::Error;

#[derive(Debug, Error)]
pub enum CreditCardError {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    BankAccount(#[from] crate::bank_account::error::BankAccountError),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),

    #[error("Negative amount")]
    NegativeAmount,

    #[error("Platfond limit reached")]
    CeilingLimitReached,
}
