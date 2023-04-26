//! This crate gives you an opinionated way of implement CQRS/event sourcing.
//!
//! Under the hood, without `postgres` feature enabled, this crate just expose some traits that can
//! be used in order to implement your version of the CQRS/event sourcing pattern. The main actors
//! are the [`Aggregate`] (with its manager), [`AggregateState`], [`EventStore`] and the [`StoreEvent`].
//!
//! The approach is to have a way, at runtime, to reload the state from an event store.
//! This means that everytime an aggregate state is needed the state should be loaded So, for example
//! while using `postgres` event store, everytime a state load is required a database query is
//! performed over the event store table.

pub use crate::esrs::aggregate::{Aggregate, AggregateManager};
pub use crate::esrs::event_handler::{EventHandler, TransactionalEventHandler};
pub use crate::esrs::policy::Policy;
pub use crate::esrs::state::AggregateState;
pub use crate::esrs::store::{EventStore, EventStoreLockGuard, StoreEvent, UnlockOnDrop};

mod esrs;

#[cfg(feature = "postgres")]
pub mod postgres {
    //! Provides implementation of the [`EventStore`] for Postgres.
    pub use crate::esrs::postgres::projector::{Projector, ProjectorPersistence};
    pub use crate::esrs::postgres::store::PgStore;
}

pub mod error {
    //! All possible errors returned by this crate
    pub use serde_json::Error as JsonError;
    #[cfg(feature = "postgres")]
    pub use sqlx::Error as SqlxError;
}

pub mod types {
    //! Provides custom types.
    pub use crate::esrs::SequenceNumber;
}
