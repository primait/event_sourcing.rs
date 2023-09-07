use serde::{Deserialize, Serialize};
use uuid::Uuid;

use esrs::Aggregate;

use crate::common::CommonError;

pub struct AggregateA;

impl Aggregate for AggregateA {
    const NAME: &'static str = "a";
    type State = i32;
    type Command = CommandA;
    type Event = EventA;
    type Error = CommonError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        Ok(vec![EventA {
            shared_id: command.shared_id,
            v: command.v,
        }])
    }

    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State {
        state + payload.v
    }
}

pub struct CommandA {
    pub v: i32,
    pub shared_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EventA {
    pub v: i32,
    pub shared_id: Uuid,
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for EventA {
    fn upcast(value: serde_json::Value, _version: Option<i32>) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}
