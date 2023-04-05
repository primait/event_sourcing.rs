use serde::de::DeserializeOwned;
use serde::Serialize;

pub mod aggregate;
pub mod policy;
pub mod state;
pub mod store;

#[cfg(feature = "postgres")]
pub mod postgres;

#[cfg(all(test, feature = "postgres"))]
mod tests;

pub type SequenceNumber = i32;

#[cfg(not(feature = "upcasting"))]
pub trait Event: Serialize + DeserializeOwned {}

#[cfg(feature = "upcasting")]
pub trait Event: Serialize + DeserializeOwned + Upcaster {}

#[cfg(feature = "upcasting")]
pub trait Upcaster
where
    Self: Sized,
{
    fn upcast(v: &serde_json::Value) -> Result<Self, serde_json::Error>;
}
