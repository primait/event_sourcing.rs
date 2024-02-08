//! Common structs shared between all the other examples

pub mod a;
pub mod b;
#[cfg(feature = "postgres")]
pub mod basic;

#[allow(dead_code)]
#[cfg(feature = "postgres")]
pub mod shared;

pub mod util;

#[derive(Debug, thiserror::Error)]
pub enum CommonError {}
