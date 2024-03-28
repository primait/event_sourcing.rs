use std::convert::TryInto;

use chrono::{DateTime, Utc};
use serde_json::Value;
use uuid::Uuid;

use crate::store::postgres::Schema;
use crate::store::StoreEvent;
use crate::types::SequenceNumber;
use serde::de::DeserializeOwned;
use serde::Serialize;

#[cfg(not(feature = "upcasting"))]
pub trait Persistable: Serialize + DeserializeOwned {}

#[cfg(not(feature = "upcasting"))]
impl<T> Persistable for T where T: Serialize + DeserializeOwned {}

#[cfg(feature = "upcasting")]
pub trait Persistable: Serialize + DeserializeOwned + crate::event::Upcaster {}

#[cfg(feature = "upcasting")]
impl<T> Persistable for T where T: Serialize + DeserializeOwned + crate::event::Upcaster {}

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
    pub fn try_into_store_event<E, S>(self) -> Result<Option<StoreEvent<E>>, serde_json::Error>
    where
        S: Schema<E>,
    {
        #[cfg(feature = "upcasting")]
        let payload = S::upcast(self.payload, self.version)?.to_event();
        #[cfg(not(feature = "upcasting"))]
        let payload = serde_json::from_value::<S>(self.payload)?.to_event();

        Ok(match payload {
            None => None,
            Some(payload) => Some(StoreEvent {
                id: self.id,
                aggregate_id: self.aggregate_id,
                payload,
                occurred_on: self.occurred_on,
                sequence_number: self.sequence_number,
                version: self.version,
            }),
        })
    }
}

impl<E: Persistable> TryInto<StoreEvent<E>> for DbEvent {
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
