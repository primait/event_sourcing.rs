use std::convert::TryInto;

use serde::{Deserialize, Serialize};
use serde_json::{Error, Value};

use esrs::event::Upcaster;
use esrs::Event;

// The actual EventB
#[derive(Event, Serialize, Deserialize, Debug)]
pub enum Event {
    Incremented(IncPayload),
    Decremented(DecPayload),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct IncPayload {
    pub u: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DecPayload {
    pub u: u64,
}

impl Upcaster for Event {
    fn upcast(value: Value) -> Result<Self, Error> {
        use serde::de::Error;

        if let Ok(event) = serde_json::from_value::<Self>(value.clone()) {
            return Ok(event);
        } else {
            let event = serde_json::from_value::<Event20230405>(value.clone())?;
            match event {
                Event20230405::Incremented(inc) => {
                    let u: u64 = inc
                        .i
                        .try_into()
                        .map_err(|_| serde_json::Error::custom("Event not serializable"))?;

                    Ok(Event::Incremented(IncPayload { u }))
                }
                Event20230405::Decremented(dec) => Ok(Event::Decremented(dec)),
            }
        }
    }
}

// Previous version of `Event`:
#[derive(Serialize, Deserialize, Debug)]
pub enum Event20230405 {
    Incremented(IncPayload20230405),
    Decremented(DecPayload),
}

// Previous version of `Event::Incremented` event payload:
#[derive(Serialize, Deserialize, Debug)]
pub struct IncPayload20230405 {
    pub i: i64,
}
