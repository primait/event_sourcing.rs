pub mod aggregate;
pub mod event;
pub mod query;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sqlite")]
pub mod sqlite;

pub type SequenceNumber = i32;
