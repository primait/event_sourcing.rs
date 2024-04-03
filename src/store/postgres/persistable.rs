use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(not(feature = "upcasting"))]
pub trait Persistable: Serialize + DeserializeOwned {}

#[cfg(not(feature = "upcasting"))]
impl<T> Persistable for T where T: Serialize + DeserializeOwned {}

#[cfg(feature = "upcasting")]
pub trait Persistable: Serialize + DeserializeOwned + crate::event::Upcaster {}

#[cfg(feature = "upcasting")]
impl<T> Persistable for T where T: Serialize + DeserializeOwned + crate::event::Upcaster {}
