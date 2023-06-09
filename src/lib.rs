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

pub use crate::esrs::aggregate::Aggregate;
#[cfg(any(feature = "kafka", feature = "rabbit"))]
pub use crate::esrs::event_bus;
pub use crate::esrs::event_handler::{EventHandler, ReplayableEventHandler, TransactionalEventHandler};
pub use crate::esrs::manager::AggregateManager;
#[cfg(feature = "postgres")]
pub use crate::esrs::postgres;
#[cfg(feature = "rebuilder")]
pub use crate::esrs::rebuilder;
pub use crate::esrs::state::AggregateState;
pub use crate::esrs::store::{EventStore, EventStoreLockGuard, StoreEvent, UnlockOnDrop};

mod esrs;

#[cfg(feature = "sql")]
pub mod sql {
    pub use crate::esrs::sql::event::Event;
    pub use crate::esrs::sql::migrations::{Migrations, MigrationsHandler};
}

pub mod types {
    //! Provides custom types.
    pub use crate::esrs::SequenceNumber;
}
