use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::blocking::store::StoreEvent;

pub trait Policy<Event: Serialize + DeserializeOwned + Clone, Error> {
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    fn handle_event(&self, event: &StoreEvent<Event>) -> Result<(), Error>;
}
