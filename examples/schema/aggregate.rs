use std::marker::PhantomData;

use esrs::Aggregate;

use crate::common::CommonError;

pub(crate) struct SchemaAggregate<EventType>(PhantomData<EventType>);

impl<EventType> Aggregate for SchemaAggregate<EventType>
where
    EventType: Default,
{
    const NAME: &'static str = "schema";
    type State = SchemaState<EventType>;
    type Command = SchemaCommand;
    type Event = EventType;
    type Error = CommonError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {}
    }

    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State {
        let mut events = state.events;
        events.push(payload);

        Self::State { events }
    }
}

#[derive(Default)]
pub(crate) struct SchemaState<EventType> {
    pub(crate) events: Vec<EventType>,
}

pub(crate) enum SchemaCommand {}
