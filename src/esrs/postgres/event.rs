use std::convert::TryInto;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde_json::Value;
use uuid::Uuid;

use crate::store::StoreEvent;
use crate::types::SequenceNumber;

#[derive(sqlx::FromRow, serde::Serialize, serde::Deserialize, Debug)]
pub struct Event {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
}

impl<Payload: DeserializeOwned> TryInto<StoreEvent<Payload>> for Event {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<StoreEvent<Payload>, Self::Error> {
        Ok(StoreEvent {
            id: self.id,
            aggregate_id: self.aggregate_id,
            payload: serde_json::from_value::<Payload>(self.payload)?,
            occurred_on: self.occurred_on,
            sequence_number: self.sequence_number,
        })
    }
}
