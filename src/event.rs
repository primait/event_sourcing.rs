use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(not(feature = "upcasting"))]
pub trait Event: Serialize + DeserializeOwned + Send + Sync {}

#[cfg(not(feature = "upcasting"))]
impl<T> Event for T where T: Serialize + DeserializeOwned + Send + Sync {}

#[cfg(feature = "upcasting")]
pub trait Event: Serialize + DeserializeOwned + Upcaster + Send + Sync {}

#[cfg(feature = "upcasting")]
impl<T> Event for T where T: Serialize + DeserializeOwned + Upcaster + Send + Sync {}

#[cfg(feature = "upcasting")]
pub trait Upcaster
where
    Self: Sized,
{
    fn upcast(value: serde_json::Value, _version: Option<i32>) -> Result<Self, serde_json::Error>
    where
        Self: DeserializeOwned,
    {
        serde_json::from_value(value)
    }

    fn current_version() -> Option<i32> {
        None
    }
}
