use serde::{Deserialize, Serialize};
use serde_json::{Error, Value};

use esrs::event::Upcaster;
use esrs::Event;

// The actual EventA
#[derive(Event, Serialize, Deserialize, Debug)]
#[serde(tag = "event_type")]
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

        // First of all try to deserialize the event using current version
        if let Ok(event) = serde_json::from_value::<Self>(value.clone()) {
            return Ok(event);
        }

        // Then trying manually
        match value {
            Value::Object(fields) => match fields.get("i") {
                None => Err(serde_json::Error::custom("Event not serializable")),
                Some(value) => match value {
                    Value::Number(n) => {
                        let u: u64 = n
                            .as_u64()
                            .ok_or_else(|| serde_json::Error::custom("Event not serializable"))?;

                        let event_type = fields
                            .get("event_type")
                            .ok_or_else(|| serde_json::Error::custom("Event not serializable"))?;

                        if let Value::String(str) = event_type {
                            match str.as_str() {
                                "Incremented" => Ok(Self::Incremented(IncPayload { u })),
                                "Decremented" => Ok(Self::Decremented(DecPayload { u })),
                                _ => Err(serde_json::Error::custom("Event not serializable")),
                            }
                        } else {
                            Err(serde_json::Error::custom("Event not serializable"))
                        }
                    }
                    _ => Err(serde_json::Error::custom("Event not serializable")),
                },
            },
            _ => Err(serde_json::Error::custom("TestEvent not serializable")),
        }
    }
}

// Old payloads for EventA was:
// pub struct IncPayload {
//     pub i: i64,
// }
