pub mod aggregate;
pub mod event_bus;
pub mod event_handler;
pub mod manager;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(any(feature = "postgres"))]
pub mod sql;

pub type SequenceNumber = i32;
