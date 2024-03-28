//! This example demonstrates how it is possible to use the schema mechanism for upcasting.
//!
//! The module `before_upcasting` represents the code in the system before the upcasting of an
//! event. The first part of the example shows the state of the system before the event is
//! upcasted.
//!
//! The module `after_upcasting` represents the code in the system after the upcasting of an
//! schema. Similarly, the second part of the example shows how the upcasting of the event will
//! play out in a running system.
//!
//! Note the only place that still required to reference the original shape of the event is in the
//! schema itself and the rest of the system can simply operate as if the event has always been of
//! this shape.

use esrs::manager::AggregateManager;
use serde::{Deserialize, Serialize};

use sqlx::PgPool;
use uuid::Uuid;

use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::store::EventStore;
use esrs::AggregateState;

use crate::common::CommonError;

pub(crate) enum Command {}

mod before_upcasting {
    //! This module represents the code of the initial iteration of the system.
    use super::*;
    pub(crate) struct Aggregate;
    impl esrs::Aggregate for Aggregate {
        const NAME: &'static str = "schema_upcasting";
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

mod after_upcasting {
    //! This module represents the code after the upcasting has been implemented
    use super::*;
    pub(crate) struct Aggregate;
    impl esrs::Aggregate for Aggregate {
        const NAME: &'static str = "schema_upcasting";
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
        B { contents: String, count: u64 },
        C { count: u64 },
    }

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
                Event::B { contents, count } => Schema::D { contents, count },
            }
        }

        fn to_event(self) -> Option<Event> {
            match self {
                Schema::A => Some(Event::A),
                Schema::B { contents } => Some(Event::B { contents, count: 1 }),
                Schema::C { count } => Some(Event::C { count }),
                Schema::D { contents, count } => Some(Event::B { contents, count }),
            }
        }
    }
}

pub(crate) async fn example(pool: PgPool) {
    let aggregate_id: Uuid = Uuid::new_v4();

    // Initial state of the system
    {
        use before_upcasting::{Aggregate, Event, Schema};

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

    // After upcasting before_upcasting::Event::B to after_upcasting::Event::B
    {
        use after_upcasting::{Aggregate, Event, Schema};

        let store: PgStore<Aggregate, _> = PgStoreBuilder::new(pool.clone())
            .with_schema::<Schema>()
            .try_build()
            .await
            .unwrap();

        let events = vec![
            Event::A,
            Event::C { count: 42 },
            Event::B {
                contents: "this is the new events".to_owned(),
                count: 21,
            },
        ];

        let manager = AggregateManager::new(store.clone());
        let mut state = manager.load(aggregate_id).await.unwrap().unwrap();
        let _ = store.persist(&mut state, events).await.unwrap();

        let persisted_events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

        // All the events are visible
        assert_eq!(persisted_events.len(), 6);

        // The events have been upcasted
        assert!(matches!(persisted_events[1], Event::B { count: 1, .. }));
    }
}
