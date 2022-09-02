pub mod aggregate;
pub mod event;
pub mod policy;
pub mod projector;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod query;

#[cfg(feature = "postgres")]
pub mod postgres;

pub type SequenceNumber = i32;
