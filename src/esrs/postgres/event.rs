use std::convert::TryInto;

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::types::SequenceNumber;
use crate::StoreEvent;

/// Event representation on the event store
#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Debug)]
pub struct PgEvent {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
}

impl<E: serde::de::DeserializeOwned> TryInto<StoreEvent<E>> for PgEvent {
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
