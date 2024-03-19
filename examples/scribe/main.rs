//! The purpose of this example is to demonstrate the process of deleting an aggregate and its
//! projections using the [`PgStore`].

use sqlx::{Pool, Postgres};

use esrs::store::postgres::{PgStore, PgStoreBuilder, PgStoreError};

#[path = "../common/lib.rs"]
mod common;

#[derive(Default)]
struct State {
    value: u8,
}
struct Event {
    changed_to: u8,
}
struct Command {
    increase_by: u8,
}
#[derive(Debug, thiserror::Error)]
enum Error {
    #[error("Overflow")]
    Overflow,
    #[error(transparent)]
    Store(#[from] PgStoreError),
}

struct Aggregate;

impl esrs::Aggregate for Aggregate {
    const NAME: &'static str = "Aggregate";

    type State = State;
    type Command = Command;
    type Event = Event;
    type Error = Error;

    fn handle_command(state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        let State { value } = state;
        let Command { increase_by } = command;

        match value.checked_add(increase_by) {
            Some(changed_to) => Ok(vec![Event { changed_to }]),
            None => Err(Error::Overflow),
        }
    }

    fn apply_event(_: Self::State, payload: Self::Event) -> Self::State {
        let Event { changed_to } = payload;

        State { value: changed_to }
    }
}

struct Scribe;

impl esrs::store::postgres::Scribe<Event> for Scribe {
    fn serialize(event: &Event) -> serde_json::Result<serde_json::Value> {
        let &Event { changed_to } = event;

        Ok(serde_json::Value::Number(changed_to.into()))
    }

    fn deserialize(value: serde_json::Value) -> serde_json::Result<Event> {
        let changed_to = serde_json::from_value(value)?;

        Ok(Event { changed_to })
    }
}

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = common::util::new_pool().await;

    let store: PgStore<Aggregate, Scribe> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let manager = esrs::manager::AggregateManager::new(store);

    let aggregate_state: esrs::AggregateState<State> = esrs::AggregateState::new();
    let aggregate_id = *aggregate_state.id();
    assert_eq!(aggregate_state.inner().value, 0);
    manager
        .handle_command::<Error>(aggregate_state, Command { increase_by: 42 })
        .await
        .expect("Should be ok");

    let aggregate_state = manager
        .load(aggregate_id)
        .await
        .expect("Should load")
        .expect("Should be found");
    assert_eq!(aggregate_state.inner().value, 42);
}
