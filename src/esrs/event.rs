use serde::de::DeserializeOwned;
use serde::Serialize;

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
