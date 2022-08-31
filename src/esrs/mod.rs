pub mod aggregate;
pub mod event;
pub mod policy;
pub mod projector;
pub mod state;
pub mod store;

#[cfg(any(feature = "postgres", feature = "sqlite"))]
mod setup;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub type SequenceNumber = i32;
