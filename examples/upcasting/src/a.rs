use esrs::event::Upcaster;
use esrs::Event;
use serde::{Deserialize, Serialize};
use serde_json::{Error, Value};

// The actual EventA
#[derive(Event, Serialize, Deserialize, Debug)]
#[serde(untagged)]
pub enum Event {
    Incremented(IncPayload),
    Decremented(DecPayload),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IncPayload {
    pub u: u32,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DecPayload {
    pub u: u32,
}

impl Upcaster for Event {
    fn upcast(v: &Value) -> Result<Self, Error> {
        todo!()
    }
}

// Old payloads for EventA
