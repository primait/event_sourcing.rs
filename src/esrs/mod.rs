pub mod aggregate;
pub mod event;
pub mod event_bus;
pub mod event_handler;
pub mod manager;
pub mod rebuilder;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;
#[cfg(feature = "sql")]
pub mod sql;

pub type SequenceNumber = i32;
