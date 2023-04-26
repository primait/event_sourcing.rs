pub mod aggregate;
pub mod event_handler;
pub mod policy;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(all(test, feature = "postgres"))]
mod tests;

pub type SequenceNumber = i32;
