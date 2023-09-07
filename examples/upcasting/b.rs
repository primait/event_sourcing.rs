use std::convert::{TryFrom, TryInto};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use esrs::event::Upcaster;
use esrs::Aggregate;

use crate::{Command, Error};

// The actual EventB. This gets serialized as:
// {"event_type": "incremented", "u": 0}
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

impl Upcaster for Event {
    fn upcast(value: Value, version: Option<i32>) -> Result<Self, serde_json::Error> {
        match version {
            None | Some(0) => Event20230405::upcast(value, version)?.try_into(),
            Some(1) => serde_json::from_value::<Self>(value),
            Some(v) => {
                use serde::de::Error;
                Err(serde_json::Error::custom(format!(
                    "Invalid event version number: {}",
                    v
                )))
            }
        }
    }

    fn current_version() -> Option<i32> {
        Some(1)
    }
}

// Previous version of `Event`:
#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum Event20230405 {
    Incremented(IncPayload20230405),
    Decremented(DecPayload),
}

// Previous version of `Event::Incremented` event payload:
#[derive(Serialize, Deserialize, Debug)]
pub struct IncPayload20230405 {
    pub i: i64,
}

impl TryFrom<Event20230405> for Event {
    type Error = serde_json::Error;

    fn try_from(event: Event20230405) -> Result<Self, Self::Error> {
        match event {
            Event20230405::Incremented(inc) => {
                use serde::de::Error;
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

// First version of event. Impl is trivial
impl Upcaster for Event20230405 {
    fn upcast(value: Value, _version: Option<i32>) -> Result<Self, serde_json::Error> {
        serde_json::from_value::<Self>(value)
    }

    fn current_version() -> Option<i32> {
        Some(1)
    }
}

/// This is the `Aggregate` where we use the `EventB`, with a versioned-event upcasting approach
pub struct AggregateB;

impl Aggregate for AggregateB {
    const NAME: &'static str = "upsert_b";
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
