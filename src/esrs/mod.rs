pub mod aggregate;
pub mod aggregate_manager;
pub mod aggregate_state;
pub mod event_bus;
pub mod event_handler;
pub mod event_store;
pub mod rebuilder;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sql")]
pub mod sql;

pub type SequenceNumber = i32;
