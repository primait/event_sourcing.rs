pub use builder::*;
pub use event_store::*;

mod builder;
mod event_store;

// Trait aliases are experimental. See issue #41517 <https://github.com/rust-lang/rust/issues/41517>
// trait PgTransactionalEventHandler<A> = TransactionalEventHandler<A, PgStoreError, PgConnection> where A: Aggregate;

#[derive(thiserror::Error, Debug)]
pub enum PgStoreError {
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Custom(Box<dyn std::error::Error + Send>),
}
