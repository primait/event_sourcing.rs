use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::blocking::store::StoreEvent;

pub trait Projector<Event: Serialize + DeserializeOwned + Clone, Error> {
    /// This function projects one event in each read model that implements this trait.
    /// The result is meant to catch generic errors.
    /// The content of the result is an Option because it is not ensured that the projection
    /// of an event generates insertion in the specific read model.
    fn project(&self, event: &StoreEvent<Event>) -> Result<(), Error>;
}
