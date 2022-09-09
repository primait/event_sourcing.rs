pub mod aggregate;
mod event;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(feature = "postgres")]
mod query;

pub type SequenceNumber = i32;
