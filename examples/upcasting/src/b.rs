use serde::{Deserialize, Serialize};
use serde_json::{Error, Value};

use esrs::event::Upcaster;
use esrs::Event;

// The actual EventB
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

// Previous version of `Event::Incremented` event payload was:
#[derive(Serialize, Deserialize, Debug)]
pub struct IncPayload20230405 {
    pub i: i32,
}

impl Upcaster for Event {
    fn upcast(v: &Value) -> Result<Self, Error> {
        todo!()
    }
}
