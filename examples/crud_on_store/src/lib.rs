//! Actual schema is defined [here](https://github.com/primait/event_sourcing.rs/blob/master/src/esrs/postgres/statement.rs#L58)
use std::convert::TryInto;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use esrs::{Aggregate, AggregateManager, StoreEvent};

// Get event by event id. The table should be the aggregate name, not the aggregate_events
pub async fn get_event_by_id<T: AggregateManager>(
    event_id: Uuid,
    executor: impl Executor<'_, Database = Postgres>,
) -> Option<StoreEvent<T::Event>>
where
    T::Event: DeserializeOwned,
    <T as Aggregate>::Event: std::fmt::Debug,
{
    // The path in the include_str! is the one for the given query. In your codebase you can copy the
    // file or directly use the content
    let query: String = format!(include_str!("../statements/select_by_event_id.sql"), table_name::<T>());

    sqlx::query_as::<_, Event>(query.as_str())
        .bind(event_id)
        .fetch_one(executor)
        .await
        .ok()
        .map(|v| v.try_into().expect("Failed to cast database event into T"))
}

// In order to implement `Get all events by aggregate id` you can either use `PgStore<Manager>::by_aggregate_id`
// function or, if there's the need to do it in a transaction, use this function.
pub async fn get_events_by_aggregate_id<T: AggregateManager>(
    aggregate_id: Uuid,
    executor: impl Executor<'_, Database = Postgres>,
) -> Vec<StoreEvent<T::Event>>
where
    T::Event: DeserializeOwned,
    <T as Aggregate>::Error: std::fmt::Debug,
{
    // The path in the include_str! is the one for the given query. In your codebase you can copy the
    // file or directly use the content
    let query: String = format!(
        include_str!("../../../src/esrs/postgres/statements/select_by_aggregate_id.sql"),
        table_name::<T>()
    );

    sqlx::query_as::<_, Event>(query.as_str())
        .bind(aggregate_id)
        .fetch_all(executor)
        .await
        .expect("Failed to get events by aggregate id")
        .into_iter()
        .map(|event| Ok(event.try_into().expect("Failed to deserialize event")))
        .collect::<Result<Vec<StoreEvent<T::Event>>, T::Error>>()
        .expect("Failed to deserialize events by aggregate id")
}

// In order to implement `Insert event` without the need to specify an event id you can use
// `PgStore<Manager>::save_event`. If there's the need to set the event id or to do it in a transaction,
// use this function.
// Note: sequence number must be larger than any sequence number in the DB for this aggregate id
pub async fn insert_event_with_given_event_id<T: AggregateManager>(
    id: Uuid,
    aggregate_id: Uuid,
    payload: &T::Event,
    occurred_on: DateTime<Utc>,
    sequence_number: i32,
    executor: impl Executor<'_, Database = Postgres>,
) where
    T::Event: Serialize,
{
    // The path in the include_str! is the one for the given query. In your codebase you can copy the
    // file or directly use the content
    let query: String = format!(
        include_str!("../../../src/esrs/postgres/statements/insert.sql"),
        table_name::<T>()
    );
    let _ = sqlx::query(query.as_str())
        .bind(id)
        .bind(aggregate_id)
        .bind(Json(&payload))
        .bind(occurred_on)
        .bind(sequence_number)
        .execute(executor)
        .await
        .expect("Failed to insert event");
}

// Update event by event id
pub async fn update_event_by_event_id<T: AggregateManager>(
    event_id: Uuid,
    new_payload: &T::Event,
    executor: impl Executor<'_, Database = Postgres>,
) where
    T::Event: Serialize,
{
    // The path in the include_str! is the one for the given query. In your codebase you can copy the
    // file or directly use the content
    let query: String = format!(
        include_str!("../statements/update_event_by_event_id.sql"),
        table_name::<T>()
    );

    sqlx::query(query.as_str())
        .bind(event_id)
        .bind(Json(new_payload))
        .execute(executor)
        .await
        .expect("Failed to update event payload by event id");
}

// Delete event by event id
pub async fn delete_event_by_event_id<T: AggregateManager>(
    event_id: Uuid,
    executor: impl Executor<'_, Database = Postgres>,
) {
    // The path in the include_str! is the one for the given query. In your codebase you can copy the
    // file or directly use the content
    let query: String = format!(
        include_str!("../statements/delete_event_by_event_id.sql"),
        table_name::<T>()
    );

    sqlx::query(query.as_str())
        .bind(event_id)
        .execute(executor)
        .await
        .expect("Failed to delete event by event id");
}

// In order to implement `Delete all events by aggregate id` you can either use `PgStore<Manager>::delete`
// function or, if there's the need to do it in a transaction, use this function.
pub async fn delete_events_by_aggregate_id<T: AggregateManager>(
    aggregate_id: Uuid,
    executor: impl Executor<'_, Database = Postgres>,
) {
    // The path in the include_str! is the one for the given query. In your codebase you can copy the
    // file or directly use the content
    let query: String = format!(
        include_str!("../../../src/esrs/postgres/statements/delete_by_aggregate_id.sql"),
        table_name::<T>()
    );

    sqlx::query(query.as_str())
        .bind(aggregate_id)
        .execute(executor)
        .await
        .expect("Failed to delete event by aggregate id");
}

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Debug)]
pub struct Event {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: i32,
}

impl<E: DeserializeOwned> TryInto<StoreEvent<E>> for Event {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<StoreEvent<E>, Self::Error> {
        Ok(StoreEvent {
            id: self.id,
            aggregate_id: self.aggregate_id,
            payload: serde_json::from_value::<E>(self.payload)?,
            occurred_on: self.occurred_on,
            sequence_number: self.sequence_number,
        })
    }
}

// esrs automatically makes a table with you events based on the aggregate name, called
// <aggregate_name>_events. This function just simulates that behaviour
fn table_name<T: AggregateManager>() -> String {
    format!("{}_events", T::name())
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use sqlx::PgPool;
    use uuid::Uuid;

    use esrs::postgres::PgStore;
    use esrs::{Aggregate, AggregateManager};

    use crate::{
        delete_event_by_event_id, delete_events_by_aggregate_id, get_event_by_id, get_events_by_aggregate_id,
        insert_event_with_given_event_id, table_name, update_event_by_event_id,
    };

    #[derive(Debug, thiserror::Error)]
    pub enum Error {
        #[error(transparent)]
        Json(#[from] esrs::error::JsonError),
        #[error(transparent)]
        Sql(#[from] esrs::error::SqlxError),
    }

    pub struct Agg;

    impl Aggregate for Agg {
        type State = ();
        type Command = ();
        type Event = Payload;
        type Error = Error;

        fn handle_command(_state: &Self::State, _command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
            todo!()
        }

        fn apply_event(_state: Self::State, _payload: Self::Event) -> Self::State {
            todo!()
        }
    }

    impl AggregateManager for Agg {
        type EventStore = PgStore<Self>;

        fn name() -> &'static str
        where
            Self: Sized,
        {
            "test"
        }

        fn event_store(&self) -> &Self::EventStore {
            todo!()
        }
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug)]
    pub struct Payload {
        value: i32,
    }

    #[sqlx::test]
    async fn crud_test(pool: PgPool) {
        // I need to create the table..
        let query: String = format!(
            include_str!("../../../src/esrs/postgres/statements/create_table.sql"),
            table_name::<Agg>()
        );
        let _ = sqlx::query(query.as_str()).execute(&pool).await.unwrap();

        // Data
        let event_id: Uuid = Uuid::new_v4();
        let aggregate_id: Uuid = Uuid::new_v4();
        let payload: Payload = Payload { value: 0 };

        // Assert that id doesn't already exist in the table
        let event = get_event_by_id::<Agg>(event_id, &pool).await;
        assert!(event.is_none());

        // Inserting the event
        insert_event_with_given_event_id::<Agg>(event_id, aggregate_id, &payload, Utc::now(), 1, &pool).await;

        // Asserting that at this time the event exists
        let event = get_event_by_id::<Agg>(event_id, &pool).await;
        assert!(event.is_some());
        assert_eq!(event.unwrap().payload.value, 0);

        // Updating the event
        let payload = Payload { value: 1 };
        update_event_by_event_id::<Agg>(event_id, &payload, &pool).await;

        // Asserting that value in the payload has been updated
        let event = get_event_by_id::<Agg>(event_id, &pool).await;
        assert!(event.is_some());
        assert_eq!(event.unwrap().payload.value, 1);

        // Deleting the event
        delete_event_by_event_id::<Agg>(event_id, &pool).await;

        // Asserting that event doesn't exist anymore
        let event = get_event_by_id::<Agg>(event_id, &pool).await;
        assert!(event.is_none());

        let aggregate_id: Uuid = Uuid::new_v4();
        // Event 1
        let event_id_1: Uuid = Uuid::new_v4();
        let payload_1: Payload = Payload { value: 1 };

        // Inserting 1st event
        insert_event_with_given_event_id::<Agg>(event_id_1, aggregate_id, &payload_1, Utc::now(), 1, &pool).await;

        // Event 2
        let event_id_2: Uuid = Uuid::new_v4();
        let payload_2: Payload = Payload { value: 2 };

        // Inserting 2nd event
        insert_event_with_given_event_id::<Agg>(event_id_2, aggregate_id, &payload_2, Utc::now(), 1, &pool).await;

        // Asserting that at this time the event exists with by_aggregate_id
        let mut events = get_events_by_aggregate_id::<Agg>(aggregate_id, &pool).await;
        assert_eq!(events.len(), 2);
        assert_eq!(events.pop().unwrap().payload.value, 2);
        assert_eq!(events.pop().unwrap().payload.value, 1);

        // Deleting the event
        delete_events_by_aggregate_id::<Agg>(aggregate_id, &pool).await;

        // Asserting that events don't exist anymore
        let event = get_events_by_aggregate_id::<Agg>(aggregate_id, &pool).await;
        assert!(event.is_empty());
    }
}
