pub use builder::*;
pub use event_store::*;
pub use schema::*;

mod builder;
mod event_store;
pub mod persistable;
mod schema;

// Trait aliases are experimental. See issue #41517 <https://github.com/rust-lang/rust/issues/41517>
// trait PgTransactionalEventHandler<A> = TransactionalEventHandler<A, PgStoreError, PgConnection> where A: Aggregate;

#[derive(thiserror::Error, Debug)]
pub enum PgStoreError {
    /// Sql error
    #[error(transparent)]
    Sqlx(#[from] sqlx::Error),
    /// Serialization/deserialization error
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    /// Error while running a TransactionalEventHandler inside of the event store.
    #[error(transparent)]
    Custom(Box<dyn std::error::Error + Send + Sync>),
}
