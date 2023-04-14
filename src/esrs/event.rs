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
    // TODO: should we want this function to have a default implementation?
    fn upcast(value: serde_json::Value) -> Result<Self, serde_json::Error>;

    fn version<T>() -> Option<T>
    where
        T: Into<u32>,
    {
        None
    }
}
