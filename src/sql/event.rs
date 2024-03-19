use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::store::StoreEvent;
use crate::types::SequenceNumber;

/// Event representation on the event store
#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Debug)]
pub struct DbEvent {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
    pub version: Option<i32>,
}

impl DbEvent {
    pub fn deserialize<E, S>(self) -> serde_json::Result<StoreEvent<E>>
    where
        S: crate::store::postgres::Scribe<E>,
    {
        Ok(StoreEvent {
            id: self.id,
            aggregate_id: self.aggregate_id,
            payload: S::deserialize(self.payload)?,
            occurred_on: self.occurred_on,
            sequence_number: self.sequence_number,
            version: self.version,
        })
    }
}
