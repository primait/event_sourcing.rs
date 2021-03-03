#[cfg(feature = "async")]
pub use async_impl::*;

#[cfg(feature = "async")]
mod async_impl;

#[cfg(feature = "blocking")]
pub mod blocking;

mod query;
pub mod state;

pub type SequenceNumber = i32;
