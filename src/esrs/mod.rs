pub mod aggregate;
pub mod event;
pub mod state;
pub mod store;

#[cfg(any(feature = "postgres", feature = "sqlite"))]
pub mod query;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub type SequenceNumber = i32;
