//! This example demonstrates several stages of the evolution of a system.
//!
//! The first part for an event store with no schema (where the `Aggregate::Event` type implement
//! `Event`.
//!
//! The next evolution of the system is the introduction of the `Schema` to the event store. This
//! removes the need for `Aggregate::Event` to implement `Event` instead relies on the
//! `From<Aggregate::Event>` and `Into<Option<Aggregate::Event>>` on `Schema` and the fact that
//! `Schema` implements event.
//!
//! The next evolution demonstrates how an event can be deprecated using the `Schema` mechanism. A
//! new event variant is introduced and one is deprecated. The use of the schema enables the rest
//! of the system to not have to concern itself with the deprecated events. The only place it still
//! exists is as a variant of the `Schema` type.
//!
//! The final part is an alternative way for the system to evolve using the `Schema` mechanism as a
//! way to upcast from one shape of and event to another.

use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::store::EventStore;
use esrs::AggregateState;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

mod aggregate;
#[path = "../common/lib.rs"]
mod common;

use crate::common::util::new_pool;
use aggregate::SchemaAggregate;

#[derive(Deserialize, Serialize)]
enum Schema {
    A,
    B { contents: String },
    C { count: u64 },
    D { contents: String, count: u64 },
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for Schema {}

mod before_schema {
    use serde::{Deserialize, Serialize};

    #[derive(Default, Deserialize, Serialize)]
    pub enum Event {
        #[default]
        A,
        B {
            contents: String,
        },
        C {
            count: u64,
        },
    }

    #[cfg(feature = "upcasting")]
    impl esrs::event::Upcaster for Event {}
}

mod after_schema {
    use super::Schema;

    #[derive(Default)]
    pub enum Event {
        #[default]
        A,
        B {
            contents: String,
        },
        C {
            count: u64,
        },
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
                Self::D { .. } => panic!("not supported"),
            }
        }
    }
}

mod with_deprecation {
    use super::Schema;

    #[derive(Default)]
    pub enum Event {
        #[default]
        A,
        C {
            count: u64,
        },
        D {
            contents: String,
            count: u64,
        },
    }

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

mod upcasting {
    use super::Schema;

    #[derive(Default)]
    pub enum Event {
        #[default]
        A,
        B {
            contents: String,
            count: u64,
        },
        C {
            count: u64,
        },
    }

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

#[tokio::main]
async fn main() {
    let pool = new_pool().await;
    let aggregate_id: Uuid = Uuid::new_v4();

    // Before the schema
    let schemaless_store: PgStore<SchemaAggregate<before_schema::Event>> =
        PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let events = vec![
        before_schema::Event::A,
        before_schema::Event::B {
            contents: "hello world".to_owned(),
        },
        before_schema::Event::C { count: 42 },
    ];

    let mut state = AggregateState::with_id(aggregate_id);
    let events = schemaless_store.persist(&mut state, events).await.unwrap();

    assert_eq!(events.len(), 3);

    // With schema before deprecation of EventA
    let old_store: PgStore<SchemaAggregate<after_schema::Event>, Schema> = PgStoreBuilder::new(pool.clone())
        .with_schema::<Schema>()
        .try_build()
        .await
        .unwrap();

    let events = vec![
        after_schema::Event::A,
        after_schema::Event::B {
            contents: "goodbye world".to_owned(),
        },
        after_schema::Event::C { count: 42 },
    ];

    let manager = AggregateManager::new(old_store.clone());
    let mut state = manager.load(aggregate_id).await.unwrap().unwrap();
    let _ = old_store.persist(&mut state, events).await.unwrap();

    let events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

    assert_eq!(events.len(), 6);

    // After deprecation of EventA and addition of EventC
    let new_store: PgStore<SchemaAggregate<with_deprecation::Event>, Schema> = PgStoreBuilder::new(pool.clone())
        .with_schema::<Schema>()
        .try_build()
        .await
        .unwrap();

    let events = new_store.by_aggregate_id(aggregate_id).await.unwrap();

    assert_eq!(events.len(), 4);

    let events = vec![
        with_deprecation::Event::A,
        with_deprecation::Event::C { count: 42 },
        with_deprecation::Event::D {
            contents: "this is the new events".to_owned(),
            count: 21,
        },
    ];

    let manager = AggregateManager::new(new_store.clone());
    let mut state = manager.load(aggregate_id).await.unwrap().unwrap();
    let _ = new_store.persist(&mut state, events).await.unwrap();

    let events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

    // The deprecated events are skipped
    assert_eq!(events.len(), 7);

    // After upcasting of EventA
    let upcasting_store: PgStore<SchemaAggregate<upcasting::Event>, Schema> = PgStoreBuilder::new(pool.clone())
        .with_schema::<Schema>()
        .try_build()
        .await
        .unwrap();
    let manager = AggregateManager::new(upcasting_store.clone());
    let events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

    // All the events are visible
    assert_eq!(events.len(), 9);

    // The events have been upcasted
    assert!(matches!(events[1], upcasting::Event::B { count: 1, .. }));
    assert!(matches!(events[4], upcasting::Event::B { count: 1, .. }));
}
