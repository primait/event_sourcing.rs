//! This crate gives you an opinionated way of implement CQRS/event sourcing.
//!
//! Under the hood, without `postgres` feature enabled, this crate just expose some traits that can
//! be used in order to implement your version of the CQRS/event sourcing pattern. The main actors
//! are the [`Aggregate`] (with its manager), [`AggregateState`], [`store::EventStore`] and the
//! [`store::StoreEvent`].
//!
//! The approach is to have a way, at runtime, to reload the state from an event store.
//! This means that everytime an aggregate state is needed the state should be loaded So, for example
//! while using `postgres` event store, everytime a state load is required a database query is
//! performed over the event store table.

pub use aggregate::Aggregate;
pub use state::AggregateState;

mod aggregate;
mod state;

pub mod bus;
pub mod handler;
pub mod manager;
pub mod store;

#[cfg(feature = "rebuilder")]
pub mod rebuilder;
#[cfg(feature = "sql")]
pub mod sql;

pub mod types {
    //! Provides custom types.
    pub type SequenceNumber = i32;
}
