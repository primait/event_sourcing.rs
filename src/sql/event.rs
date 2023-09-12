use std::convert::TryInto;

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::event::Event;
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

impl<E: Event> TryInto<StoreEvent<E>> for DbEvent {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<StoreEvent<E>, Self::Error> {
        Ok(StoreEvent {
            id: self.id,
            aggregate_id: self.aggregate_id,
            #[cfg(feature = "upcasting")]
            payload: E::upcast(self.payload, self.version)?,
            #[cfg(not(feature = "upcasting"))]
            payload: serde_json::from_value::<E>(self.payload)?,
            occurred_on: self.occurred_on,
            sequence_number: self.sequence_number,
            version: self.version,
        })
    }
}
