//! This example demonstrates how it is possible to add a schema to a store.
//!
//! The module `before_schema` represents the code in the system before the introduction of the
//! schema. The first part of the example shows the state of the system before the schema is
//! introduced.
//!
//! The module `after_schema` represents the code in the system after the introduction of the
//! schema. Similarly, the second part of the example shows how the to setup the PgStore in this
//! state and how the previously persisted events are visible via the schema.
//!
//! Note that the introduction of the schema removes the need for `Aggregate::Event` to implement
//! `Persistable` instead relies on the implementation of `Schema` and the fact that `Schema`
//! implements `Persistable`.

use esrs::manager::AggregateManager;
use serde::{Deserialize, Serialize};

use sqlx::PgPool;
use uuid::Uuid;

use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::store::EventStore;
use esrs::AggregateState;

use crate::common::CommonError;

pub(crate) enum Command {}

mod before_schema {
    //! This module represents the code of the initial iteration of the system before the
    //! introduction of the schema.
    use super::*;
    pub(crate) struct Aggregate;

    impl esrs::Aggregate for Aggregate {
        const NAME: &'static str = "introducing_schema";
        type State = SchemaState;
        type Command = Command;
        type Event = Event;
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
    pub(crate) struct SchemaState {
        pub(crate) events: Vec<Event>,
    }

    // Required only before the schema is introduced, after the schema is introduced
    // we do not need to make the type serializable. See other schema examples.
    #[derive(Deserialize, Serialize)]
    pub enum Event {
        A,
        B { contents: String },
        C { count: u64 },
    }

    #[cfg(feature = "upcasting")]
    impl esrs::sql::event::Upcaster for Event {}
}

mod after_schema {
    //! This module represents the code after the introduction of the schema.
    use super::*;
    pub(crate) struct Aggregate;

    impl esrs::Aggregate for Aggregate {
        const NAME: &'static str = "introducing_schema";
        type State = SchemaState;
        type Command = Command;
        type Event = Event;
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
    pub(crate) struct SchemaState {
        pub(crate) events: Vec<Event>,
    }

    pub enum Event {
        A,
        B { contents: String },
        C { count: u64 },
    }
    #[derive(Deserialize, Serialize)]
    pub enum Schema {
        A,
        B { contents: String },
        C { count: u64 },
    }

    impl esrs::store::postgres::Schema<Event> for Schema {
        fn from_event(value: Event) -> Self {
            match value {
                Event::A => Schema::A,
                Event::B { contents } => Schema::B { contents },
                Event::C { count } => Schema::C { count },
            }
        }

        fn to_event(self) -> Option<Event> {
            match self {
                Self::A => Some(Event::A),
                Self::B { contents } => Some(Event::B { contents }),
                Self::C { count } => Some(Event::C { count }),
            }
        }
    }

    #[cfg(feature = "upcasting")]
    impl esrs::sql::event::Upcaster for Schema {}
}

pub(crate) async fn example(pool: PgPool) {
    let aggregate_id: Uuid = Uuid::new_v4();

    // Initial state of system before introduction of schema
    {
        use before_schema::{Aggregate, Event};

        let store: PgStore<Aggregate> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

        let events = vec![
            Event::A,
            Event::B {
                contents: "hello world".to_owned(),
            },
            Event::C { count: 42 },
        ];

        let mut state = AggregateState::with_id(aggregate_id);
        let events = store.persist(&mut state, events).await.unwrap();

        assert_eq!(events.len(), 3);
    }

    // After introducing Schema
    {
        use after_schema::{Aggregate, Event, Schema};

        let store: PgStore<Aggregate, Schema> = PgStoreBuilder::new(pool.clone())
            .with_schema::<Schema>()
            .try_build()
            .await
            .unwrap();

        let events = vec![
            Event::A,
            Event::B {
                contents: "goodbye world".to_owned(),
            },
            Event::C { count: 42 },
        ];

        let manager = AggregateManager::new(store.clone());
        let mut state = manager.load(aggregate_id).await.unwrap().unwrap();
        let _ = store.persist(&mut state, events).await.unwrap();

        let events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

        assert_eq!(events.len(), 6);
    }
}
