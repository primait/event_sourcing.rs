use serde::de::DeserializeOwned;

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
