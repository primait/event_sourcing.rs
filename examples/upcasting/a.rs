use serde::{Deserialize, Serialize};
use serde_json::Value;

use esrs::sql::event::Upcaster;
use esrs::Aggregate;

use crate::{Command, Error};

// The actual EventA. This gets serialized as:
// // {"event_type": "incremented", "u": 0}
#[derive(Serialize, Deserialize, PartialEq, Debug)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum Event {
    Incremented(IncPayload),
    Decremented(DecPayload),
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct IncPayload {
    pub u: u64,
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
pub struct DecPayload {
    pub u: u64,
}

// Old payload for EventA was:
// pub struct IncPayload {
//     pub i: i64,
// }

impl Upcaster for Event {
    fn upcast(value: Value, version: Option<i32>) -> Result<Self, serde_json::Error> {
        use serde::de::Error;

        match version {
            None | Some(0) => match value {
                Value::Object(fields) => match fields.get("i") {
                    None => Err(serde_json::Error::custom("Event not deserializable: missing `i` field")),
                    Some(value) => match value {
                        Value::Number(n) => {
                            let u: u64 = n.as_u64().ok_or_else(|| {
                                serde_json::Error::custom("Event not deserializable: cannot cast `i` value")
                            })?;

                            let event_type = fields.get("event_type").ok_or_else(|| {
                                serde_json::Error::custom("Event not deserializable: missing `event_type` field")
                            })?;

                            if let Value::String(str) = event_type {
                                match str.as_str() {
                                    "incremented" => Ok(Self::Incremented(IncPayload { u })),
                                    "decremented" => Ok(Self::Decremented(DecPayload { u })),
                                    _ => Err(serde_json::Error::custom("Event not deserializable: variant not found")),
                                }
                            } else {
                                Err(serde_json::Error::custom("Event not deserializable"))
                            }
                        }
                        _ => Err(serde_json::Error::custom("Event not deserializable")),
                    },
                },
                _ => Err(serde_json::Error::custom("TestEvent not deserializable")),
            },
            Some(1) => serde_json::from_value::<Self>(value),
            Some(v) => Err(serde_json::Error::custom(format!(
                "Invalid event version number: {}",
                v
            ))),
        }
    }

    fn current_version() -> Option<i32> {
        Some(1)
    }
}

/// This is the `Aggregate` where we use the `a::Event`, with a json upcasting approach
pub struct AggregateA;

impl Aggregate for AggregateA {
    const NAME: &'static str = "upsert_a";
    type State = ();
    type Command = Command;
    type Event = Event;
    type Error = Error;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::Increment { u } => Ok(vec![Self::Event::Incremented(IncPayload { u })]),
            Self::Command::Decrement { u } => Ok(vec![Self::Event::Decremented(DecPayload { u })]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        state
    }
}
