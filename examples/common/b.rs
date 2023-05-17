use serde::{Deserialize, Serialize};
use uuid::Uuid;

use esrs::Aggregate;

use crate::Error;

pub struct AggregateB;

impl Aggregate for AggregateB {
    const NAME: &'static str = "b";
    type State = i32;
    type Command = CommandB;
    type Event = EventB;
    type Error = Error;

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

#[derive(Serialize, Deserialize)]
pub struct EventB {
    pub v: i32,
    pub shared_id: Uuid,
}
