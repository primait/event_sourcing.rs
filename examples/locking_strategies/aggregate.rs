use esrs::postgres::PgStore;
use esrs::{Aggregate, AggregateManager};

pub struct FooAggregate;

impl Aggregate for FooAggregate {
    type State = ();
    type Command = FooCommand;
    type Event = FooEvent;
    type Error = FooError;

    fn handle_command(_state: &Self::State, _command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        todo!()
    }

    fn apply_event(_state: Self::State, _payload: Self::Event) -> Self::State {
        todo!()
    }
}

impl AggregateManager for FooAggregate {
    type EventStore = PgStore<Self>;

    fn name() -> &'static str
    where
        Self: Sized,
    {
        todo!()
    }

    fn event_store(&self) -> &Self::EventStore {
        todo!()
    }
}

pub struct FooCommand;

#[derive(esrs::Event, serde::Serialize, serde::Deserialize)]
pub struct FooEvent;

#[derive(thiserror::Error, Debug)]
pub enum FooError {
    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}
