use std::convert::TryInto;

use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use crate::esrs::store::StoreEvent;
use crate::esrs::SequenceNumber;

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
pub struct Event {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub occurred_on: DateTime<Utc>,
    pub sequence_number: SequenceNumber,
}

impl<E: Serialize + DeserializeOwned + Clone + Send + Sync> TryInto<StoreEvent<E>> for Event {
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
