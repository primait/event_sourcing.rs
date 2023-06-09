//! Common structs shared between all the other examples

use thiserror::Error;

pub use a::*;
pub use b::*;
#[cfg(feature = "postgres")]
pub use basic::*;
#[cfg(feature = "postgres")]
pub use shared::*;
pub use util::*;

mod a;
mod b;
#[cfg(feature = "postgres")]
mod basic;

#[allow(dead_code)]
#[cfg(feature = "postgres")]
mod shared;

mod util;

#[derive(Debug, Error)]
pub enum CommonError {
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
