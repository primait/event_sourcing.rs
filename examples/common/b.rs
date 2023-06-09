use serde::{Deserialize, Serialize};
use uuid::Uuid;

use esrs::Aggregate;

use crate::common::CommonError;

pub struct AggregateB;

impl Aggregate for AggregateB {
    const NAME: &'static str = "b";
    type State = i32;
    type Command = CommandB;
    type Event = EventB;
    type Error = CommonError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        Ok(vec![EventB {
            shared_id: command.shared_id,
            v: command.v,
        }])
    }

    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State {
        state + payload.v
    }
}

pub struct CommandB {
    pub v: i32,
    pub shared_id: Uuid,
}

#[derive(Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "upcasting", derive(esrs::Event))]
pub struct EventB {
    pub v: i32,
    pub shared_id: Uuid,
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for EventB {
    fn upcast(value: serde_json::Value, _version: Option<i32>) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}
