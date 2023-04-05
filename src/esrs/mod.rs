use serde::de::DeserializeOwned;

pub mod aggregate;
pub mod policy;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(all(test, feature = "postgres"))]
mod tests;

pub type SequenceNumber = i32;

pub trait Event: DeserializeOwned {}
