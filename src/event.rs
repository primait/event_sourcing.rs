use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(not(feature = "upcasting"))]
pub trait Event: Serialize + DeserializeOwned {}

#[cfg(not(feature = "upcasting"))]
impl<T> Event for T where T: Serialize + DeserializeOwned {}

#[cfg(feature = "upcasting")]
pub trait Event: Serialize + DeserializeOwned + Upcaster {}

#[cfg(feature = "upcasting")]
impl<T> Event for T where T: Serialize + DeserializeOwned + Upcaster {}

#[cfg(feature = "upcasting")]
pub trait Upcaster
where
    Self: Sized,
{
    // TODO: should we want this function to have a default implementation?
    fn upcast(value: serde_json::Value, version: Option<i32>) -> Result<Self, serde_json::Error>;

    fn current_version() -> Option<i32> {
        None
    }
}
