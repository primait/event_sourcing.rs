use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::store::postgres::Scribe;
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
    pub fn to_store_event<E, S>(self) -> Result<Option<StoreEvent<E>>, serde_json::Error>
    where
        S: Scribe<E>,
    {
        Ok(match S::deserialize(self.payload)? {
            None => None,
            Some(event) => Some(StoreEvent {
                id: self.id,
                aggregate_id: self.aggregate_id,
                #[cfg(feature = "upcasting")]
                payload: E::upcast(self.payload, self.version)?,
                #[cfg(not(feature = "upcasting"))]
                payload: event,
                occurred_on: self.occurred_on,
                sequence_number: self.sequence_number,
                version: self.version,
            }),
        })
    }
}
