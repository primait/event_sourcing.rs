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
use std::marker::PhantomData;

use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::store::EventStore;
use esrs::{Aggregate, AggregateState};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[path = "../common/lib.rs"]
mod common;

use crate::common::util::new_pool;
use crate::common::CommonError;

struct SchemaAggregate<EventType>(PhantomData<EventType>);

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
struct SchemaState<EventType> {
    events: Vec<EventType>,
}

enum SchemaCommand {}

#[derive(Default)]
enum SchemaEventUpcasted {
    #[default]
    EmptyEvent,
    EventA {
        contents: String,
        count: u64,
    },
    EventB {
        count: u64,
    },
}

#[derive(Default)]
enum SchemaEvent {
    #[default]
    EmptyEvent,
    EventB {
        count: u64,
    },
    EventC {
        contents: String,
        count: u64,
    },
}

#[derive(Default, Deserialize, Serialize)]
enum SchemaEventNoSchema {
    #[default]
    EmptyEvent,
    EventA {
        contents: String,
    },
    EventB {
        count: u64,
    },
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for SchemaEventNoSchema {}

#[derive(Default)]
enum SchemaEventOld {
    #[default]
    EmptyEvent,
    EventA {
        contents: String,
    },
    EventB {
        count: u64,
    },
}

#[derive(Deserialize, Serialize)]
enum Schema {
    EmptyEvent,
    EventA { contents: String },
    EventB { count: u64 },
    EventC { contents: String, count: u64 },
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for Schema {}

impl From<SchemaEvent> for Schema {
    fn from(value: SchemaEvent) -> Self {
        match value {
            SchemaEvent::EmptyEvent => Schema::EmptyEvent,
            SchemaEvent::EventB { count } => Schema::EventB { count },
            SchemaEvent::EventC { contents, count } => Schema::EventC { contents, count },
        }
    }
}

impl From<SchemaEventUpcasted> for Schema {
    fn from(value: SchemaEventUpcasted) -> Self {
        match value {
            SchemaEventUpcasted::EmptyEvent => Schema::EmptyEvent,
            SchemaEventUpcasted::EventB { count } => Schema::EventB { count },
            SchemaEventUpcasted::EventA { contents, count } => Schema::EventC { contents, count },
        }
    }
}

impl From<SchemaEventOld> for Schema {
    fn from(value: SchemaEventOld) -> Self {
        match value {
            SchemaEventOld::EmptyEvent => Schema::EmptyEvent,
            SchemaEventOld::EventA { contents } => Schema::EventA { contents },
            SchemaEventOld::EventB { count } => Schema::EventB { count },
        }
    }
}

impl From<Schema> for Option<SchemaEventOld> {
    fn from(val: Schema) -> Self {
        match val {
            Schema::EmptyEvent => Some(SchemaEventOld::EmptyEvent),
            Schema::EventA { contents } => Some(SchemaEventOld::EventA { contents }),
            Schema::EventB { count } => Some(SchemaEventOld::EventB { count }),
            Schema::EventC { .. } => panic!("not supported"),
        }
    }
}

impl From<Schema> for Option<SchemaEvent> {
    fn from(val: Schema) -> Self {
        match val {
            Schema::EmptyEvent => Some(SchemaEvent::EmptyEvent),
            Schema::EventA { .. } => None,
            Schema::EventB { count } => Some(SchemaEvent::EventB { count }),
            Schema::EventC { contents, count } => Some(SchemaEvent::EventC { contents, count }),
        }
    }
}

impl From<Schema> for Option<SchemaEventUpcasted> {
    fn from(val: Schema) -> Self {
        match val {
            Schema::EmptyEvent => Some(SchemaEventUpcasted::EmptyEvent),
            Schema::EventA { contents } => Some(SchemaEventUpcasted::EventA { contents, count: 1 }),
            Schema::EventB { count } => Some(SchemaEventUpcasted::EventB { count }),
            Schema::EventC { contents, count } => Some(SchemaEventUpcasted::EventA { contents, count }),
        }
    }
}

#[tokio::main]
async fn main() {
    let pool = new_pool().await;
    let aggregate_id: Uuid = Uuid::new_v4();

    // Before the schema
    let schemaless_store: PgStore<SchemaAggregate<SchemaEventNoSchema>> =
        PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let events = vec![
        SchemaEventNoSchema::EmptyEvent,
        SchemaEventNoSchema::EventA {
            contents: "this is soon to be deprecated".to_owned(),
        },
        SchemaEventNoSchema::EventB { count: 42 },
    ];

    let mut state = AggregateState::with_id(aggregate_id);
    let events = schemaless_store.persist(&mut state, events).await.unwrap();

    assert_eq!(events.len(), 3);

    // With schema before deprecation of EventA
    let old_store: PgStore<SchemaAggregate<SchemaEventOld>, Schema> = PgStoreBuilder::new(pool.clone())
        .with_schema::<Schema>()
        .try_build()
        .await
        .unwrap();

    let events = vec![
        SchemaEventOld::EmptyEvent,
        SchemaEventOld::EventA {
            contents: "this is soon to be deprecated".to_owned(),
        },
        SchemaEventOld::EventB { count: 42 },
    ];

    let manager = AggregateManager::new(old_store.clone());
    let mut state = manager.load(aggregate_id).await.unwrap().unwrap();
    let _ = old_store.persist(&mut state, events).await.unwrap();

    let events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

    assert_eq!(events.len(), 6);

    // After deprecation of EventA and addition of EventC
    let new_store: PgStore<SchemaAggregate<SchemaEvent>, Schema> = PgStoreBuilder::new(pool.clone())
        .with_schema::<Schema>()
        .try_build()
        .await
        .unwrap();

    let events = new_store.by_aggregate_id(aggregate_id).await.unwrap();

    assert_eq!(events.len(), 4);

    let events = vec![
        SchemaEvent::EmptyEvent,
        SchemaEvent::EventB { count: 42 },
        SchemaEvent::EventC {
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
    let upcasting_store: PgStore<SchemaAggregate<SchemaEventUpcasted>, Schema> = PgStoreBuilder::new(pool.clone())
        .with_schema::<Schema>()
        .try_build()
        .await
        .unwrap();
    let manager = AggregateManager::new(upcasting_store.clone());
    let events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

    // All the events are visible
    assert_eq!(events.len(), 9);

    // The events have been upcasted
    assert!(matches!(events[1], SchemaEventUpcasted::EventA { count: 1, .. }));
    assert!(matches!(events[4], SchemaEventUpcasted::EventA { count: 1, .. }));
}
