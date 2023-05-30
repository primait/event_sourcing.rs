use chrono::Utc;
use serde_json::json;
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use thiserror::Error;
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::EventStore;

use crate::util::new_pool;

mod a;
mod b;
#[path = "../common/util.rs"]
mod util;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let store_a: PgStore<a::AggregateA> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();
    let store_b: PgStore<b::AggregateB> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let event_id_a_1: Uuid = Uuid::new_v4();
    let event_id_a_2: Uuid = Uuid::new_v4();
    let aggregate_id_a: Uuid = Uuid::new_v4();

    // Deserialized using older version
    insert_event(
        store_a.table_name(),
        event_id_a_1,
        aggregate_id_a,
        json!({"event_type": "incremented", "i": 10}),
        1,
        Some(0),
        &pool,
    )
    .await;

    // Deserialized using latest version
    insert_event(
        store_a.table_name(),
        event_id_a_2,
        aggregate_id_a,
        json!({"event_type": "incremented", "u": 11}),
        2,
        Some(1),
        &pool,
    )
    .await;

    let event_id_b_1: Uuid = Uuid::new_v4();
    let event_id_b_2: Uuid = Uuid::new_v4();
    let aggregate_id_b: Uuid = Uuid::new_v4();

    // Deserialized using older version
    insert_event(
        store_b.table_name(),
        event_id_b_1,
        aggregate_id_b,
        json!({"event_type": "incremented", "i": 10}),
        1,
        Some(0),
        &pool,
    )
    .await;

    // Deserialized using latest version
    insert_event(
        store_b.table_name(),
        event_id_b_2,
        aggregate_id_b,
        json!({"event_type": "incremented", "u": 11}),
        2,
        Some(1),
        &pool,
    )
    .await;

    let events_a = store_a.by_aggregate_id(aggregate_id_a).await.unwrap();
    let event_a_1 = events_a.iter().find(|v| v.id == event_id_a_1).unwrap();
    let event_a_2 = events_a.iter().find(|v| v.id == event_id_a_2).unwrap();

    assert_eq!(event_a_1.payload, a::Event::Incremented(a::IncPayload { u: 10 }));
    assert_eq!(event_a_2.payload, a::Event::Incremented(a::IncPayload { u: 11 }));

    let events_b = store_b.by_aggregate_id(aggregate_id_b).await.unwrap();
    let event_b_1 = events_b.iter().find(|v| v.id == event_id_b_1).unwrap();
    let event_b_2 = events_b.iter().find(|v| v.id == event_id_b_2).unwrap();

    assert_eq!(event_b_1.payload, b::Event::Incremented(b::IncPayload { u: 10 }));
    assert_eq!(event_b_2.payload, b::Event::Incremented(b::IncPayload { u: 11 }));
}

// A simple error enum for event processing errors
#[derive(Debug, Error)]
pub enum Error {
    #[error("[Err {0}] {1}")]
    Domain(i32, String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}

// The commands received by the application, which will produce the events
pub enum Command {
    Increment { u: u64 },
    Decrement { u: u64 },
}

async fn insert_event(
    table_name: &str,
    event_id: Uuid,
    aggregate_id: Uuid,
    payload: serde_json::Value,
    sequence_number: i32,
    version: Option<i64>,
    pool: &Pool<Postgres>,
) {
    let query: String = format!(
        include_str!("../../src/esrs/sql/postgres/statements/insert.sql"),
        table_name
    );

    let _ = sqlx::query(query.as_str())
        .bind(event_id)
        .bind(aggregate_id)
        .bind(Json(&payload))
        .bind(Utc::now())
        .bind(sequence_number)
        // Version
        .bind(version)
        .execute(pool)
        .await
        .unwrap();
}
