//! This example demonstrates how it is possible to deprecate an event using the Schema mechanism
//!
//! The module `before_deprecation` represents the code in the system before the deprecation of an
//! event. The first part of the example shows the state of the system before the event is
//! deprecated.
//!
//! The module `after_deprecation` represents the code in the system after the deprecation of an
//! schema. Similarly, the second part of the example shows how the deprecation of the event will
//! play out in a running system.
//!
//! Note the only place that still required to reference the deprecated event is in the schema type
//! itself, the rest of the system can simply operate as if it never existed.

use esrs::manager::AggregateManager;
use serde::{Deserialize, Serialize};

use sqlx::PgPool;
use uuid::Uuid;

use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::store::EventStore;
use esrs::AggregateState;

use crate::common::CommonError;

pub(crate) enum Command {}

mod before_deprecation {
    //! This module represents the code of the initial iteration of the system.
    use super::*;
    pub(crate) struct Aggregate;
    impl esrs::Aggregate for Aggregate {
        const NAME: &'static str = "schema_deprecation";
        type State = State;
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
    pub(crate) struct State {
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

    #[cfg(feature = "upcasting")]
    impl esrs::event::Upcaster for Schema {}

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
}

mod after_deprecation {
    //! This module represents the code after `Event::B` has been deprecated and a new event
    //! (`Event::C`) has been introduced
    use super::*;

    pub(crate) struct Aggregate;
    impl esrs::Aggregate for Aggregate {
        const NAME: &'static str = "schema_deprecation";
        type State = State;
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
    pub(crate) struct State {
        pub(crate) events: Vec<Event>,
    }

    // Event B does not live here
    pub enum Event {
        A,
        C { count: u64 },
        D { contents: String, count: u64 },
    }

    // Event B exists here just for deserialization
    #[derive(Deserialize, Serialize)]
    pub enum Schema {
        A,
        B { contents: String },
        C { count: u64 },
        D { contents: String, count: u64 },
    }

    #[cfg(feature = "upcasting")]
    impl esrs::event::Upcaster for Schema {}

    impl esrs::store::postgres::Schema<Event> for Schema {
        fn from_event(value: Event) -> Self {
            match value {
                Event::A => Schema::A,
                Event::C { count } => Schema::C { count },
                Event::D { contents, count } => Schema::D { contents, count },
            }
        }

        fn to_event(self) -> Option<Event> {
            match self {
                Self::A => Some(Event::A),
                Self::B { .. } => None,
                Self::C { count } => Some(Event::C { count }),
                Self::D { contents, count } => Some(Event::D { contents, count }),
            }
        }
    }
}

pub(crate) async fn example(pool: PgPool) {
    let aggregate_id: Uuid = Uuid::new_v4();

    // Initial state of system
    {
        use before_deprecation::{Aggregate, Event, Schema};

        let store: PgStore<Aggregate, _> = PgStoreBuilder::new(pool.clone())
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

        let mut state = AggregateState::with_id(aggregate_id);
        let events = store.persist(&mut state, events).await.unwrap();

        assert_eq!(events.len(), 3);
    }

    // After deprecation
    {
        use after_deprecation::{Aggregate, Event, Schema};

        let store: PgStore<Aggregate, _> = PgStoreBuilder::new(pool.clone())
            .with_schema::<Schema>()
            .try_build()
            .await
            .unwrap();

        let events = store.by_aggregate_id(aggregate_id).await.unwrap();

        assert_eq!(events.len(), 2);

        let events = vec![
            Event::A,
            Event::C { count: 42 },
            Event::D {
                contents: "this is the new events".to_owned(),
                count: 21,
            },
        ];

        let manager = AggregateManager::new(store.clone());
        let mut state = manager.load(aggregate_id).await.unwrap().unwrap();
        let _ = store.persist(&mut state, events).await.unwrap();

        let events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

        // The deprecated events are skipped
        assert_eq!(events.len(), 5);
    }
}
