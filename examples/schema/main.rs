use std::marker::PhantomData;

use esrs::manager::AggregateManager;
use esrs::store::postgres::{PgStore, PgStoreBuilder};
use esrs::store::EventStore;
use esrs::Aggregate;
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

#[derive(Default, Debug)]
enum SchemaEventUpcasted {
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

#[derive(Default, Debug)]
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
            SchemaEventUpcasted::EventC { contents, count } => Schema::EventC { contents, count },
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
            Schema::EventA { contents } => Some(SchemaEventUpcasted::EventC { contents, count: 1 }),
            Schema::EventB { count } => Some(SchemaEventUpcasted::EventB { count }),
            Schema::EventC { contents, count } => Some(SchemaEventUpcasted::EventC { contents, count }),
        }
    }
}

#[tokio::main]
async fn main() {
    let pool = new_pool().await;
    let aggregate_id: Uuid = Uuid::new_v4();

    // Before deprecation of EventA
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
    let events = old_store.persist(&mut state, events).await.unwrap();

    assert_eq!(events.len(), 3);

    // After deprecation of EventA and addition of EventC
    let new_store: PgStore<SchemaAggregate<SchemaEvent>, Schema> = PgStoreBuilder::new(pool.clone())
        .with_schema::<Schema>()
        .try_build()
        .await
        .unwrap();

    let events = new_store.by_aggregate_id(aggregate_id).await.unwrap();

    assert_eq!(events.len(), 2);

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

    // The deprecated event is skipped
    assert_eq!(events.len(), 5);

    // After upcasting of EventA to EventC
    let upcasting_store: PgStore<SchemaAggregate<SchemaEventUpcasted>, Schema> = PgStoreBuilder::new(pool.clone())
        .with_schema::<Schema>()
        .try_build()
        .await
        .unwrap();
    let manager = AggregateManager::new(upcasting_store.clone());
    let events = manager.load(aggregate_id).await.unwrap().unwrap().into_inner().events;

    // All the events are visible
    assert_eq!(events.len(), 6);

    // The event has been upcasted
    assert!(matches!(events[1], SchemaEventUpcasted::EventC { count: 1, .. }));
}
