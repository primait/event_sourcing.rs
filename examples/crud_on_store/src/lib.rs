//! Actual schema is defined [here](https://github.com/primait/event_sourcing.rs/blob/master/src/esrs/postgres/statement.rs#L58)
use std::convert::TryInto;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sqlx::types::Json;
use sqlx::{Executor, Postgres};
use uuid::Uuid;

use esrs::StoreEvent;

// Get all events by aggregate id: use PgStore<Manager>::by_aggregate_id function

// Delete all events by aggregate id: use PgStore<Manager>::delete function

// Insert event: use PgStore<Manager>::save_event function

// Insert event with given event id. The table should be the aggregate name, not the aggregate_events
pub async fn insert_event_with_given_event_id<T: Serialize + Sync>(
    table: &str,
    id: Uuid,
    aggregate_id: Uuid,
    payload: &T,
    occurred_on: DateTime<Utc>,
    sequence_number: i32,
    executor: impl Executor<'_, Database = Postgres>,
) {
    // The path in the include_str! is the one for the given query. In your codebase you can copy the
    // file or directly use the content
    let name: String = format!("{}_events", table);
    let query: String = format!(include_str!("../../../src/esrs/postgres/statements/insert.sql"), name);
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

// Get event by event id. The table should be the aggregate name, not the aggregate_events
pub async fn get_event_by_id<T: DeserializeOwned>(
    table: &str,
    event_id: Uuid,
    executor: impl Executor<'_, Database = Postgres>,
) -> Option<StoreEvent<T>> {
    let name: String = format!("{}_events", table);
    let query: String = format!(include_str!("../statements/select_by_event_id.sql"), name);

    sqlx::query_as::<_, Event>(query.as_str())
        .bind(event_id)
        .fetch_one(executor)
        .await
        .ok()
        .map(|v| v.try_into().expect("Failed to cast database event into T"))
}

// Update event by event id
pub async fn update_event_by_event_id<T: Serialize + Sync>(
    table: &str,
    event_id: Uuid,
    new_payload: &T,
    executor: impl Executor<'_, Database = Postgres>,
) {
    let name: String = format!("{}_events", table);
    let query: String = format!(include_str!("../statements/update_event_by_event_id.sql"), name);

    sqlx::query(query.as_str())
        .bind(event_id)
        .bind(Json(new_payload))
        .execute(executor)
        .await
        .expect("Failed to update event payload by event id");
}

// Delete event by event id
pub async fn delete_event_by_event_id(
    table: &str,
    event_id: Uuid,
    executor: impl Executor<'_, Database = Postgres>,
) {
    let name: String = format!("{}_events", table);
    let query: String = format!(include_str!("../statements/delete_event_by_event_id.sql"), name);

    sqlx::query(query.as_str())
        .bind(event_id)
        .execute(executor)
        .await
        .expect("Failed to delete event by event id");
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

#[cfg(test)]
mod tests {
    use crate::{delete_event_by_event_id, get_event_by_id, insert_event_with_given_event_id, update_event_by_event_id};
    use chrono::{DateTime, Utc};
    use sqlx::postgres::PgPoolOptions;
    use sqlx::PgPool;
    use std::path::PathBuf;
    use std::time::Duration;
    use uuid::Uuid;

    #[derive(serde::Serialize, serde::Deserialize)]
    pub struct Payload {
        value: i32,
    }

    #[sqlx::test]
    async fn crud_test(pool: PgPool) {
        // I need to create the table..
        let name: &str = "test";
        let table_name: String = format!("{}_events", name);
        let query: String = format!(
            include_str!("../../../src/esrs/postgres/statements/create_table.sql"),
            table_name
        );
        let _ = sqlx::query(query.as_str()).execute(&pool).await.unwrap();

        // Data
        let event_id: Uuid = Uuid::new_v4();
        let aggregate_id: Uuid = Uuid::new_v4();
        let payload: Payload = Payload { value: 0 };

        // Assert that id doesn't already exist in the table
        let event = get_event_by_id::<Payload>(name, event_id, &pool).await;
        assert!(event.is_none());

        // Inserting the event
        insert_event_with_given_event_id(name, event_id, aggregate_id, &payload, Utc::now(), 1, &pool).await;

        // Asserting that at this time the event exists
        let event = get_event_by_id::<Payload>(name, event_id, &pool).await;
        assert!(event.is_some());
        assert_eq!(event.unwrap().payload.value, 0);

        // Updating the event
        let payload = Payload { value: 1 };
        update_event_by_event_id(name, event_id, &payload, &pool).await;

        // Asserting that value in the payload has been updated
        let event = get_event_by_id::<Payload>(name, event_id, &pool).await;
        assert!(event.is_some());
        assert_eq!(event.unwrap().payload.value, 1);

        // Deleting the event
        let payload = Payload { value: 1 };
        delete_event_by_event_id(name, event_id, &pool).await;

        // Asserting that event doesn't exist anymore
        let event = get_event_by_id::<Payload>(name, event_id, &pool).await;
        assert!(event.is_none());
    }
}
