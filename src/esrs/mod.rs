pub mod aggregate;
mod event;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(test)]
mod tests;

pub type SequenceNumber = i32;
