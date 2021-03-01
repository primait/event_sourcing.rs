use std::convert::TryInto;

use async_trait::async_trait;
use chrono::{NaiveDateTime, Utc};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::postgres::{PgDone, PgPoolOptions};
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::projector::Projector;
use crate::store::StoreEvent;
use crate::SequenceNumber;

use super::EventStore;

pub struct PostgreStore<
    Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
    Err: From<sqlx::Error> + From<serde_json::Error>,
> {
    pool: Pool<Postgres>,
    select: String,
    insert: String,
    projectors: Vec<Box<dyn Projector<Evt, Err> + Send + Sync>>,
}

impl<
        'a,
        Evt: 'a + Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    > PostgreStore<Evt, Err>
{
    pub async fn new(
        url: &'a str,
        name: &'a str,
        projectors: Vec<Box<dyn Projector<Evt, Err> + Send + Sync>>,
    ) -> Result<Self, Err> {
        let pool: Pool<Postgres> = PgPoolOptions::new().connect(url).await?;
        // Check if table and indexes exist and eventually create them
        let _ = run_preconditions(&pool, name).await?;

        Ok(Self {
            pool: PgPoolOptions::new().connect(url).await?,
            select: select_query(name),
            insert: insert_query(name),
            projectors,
        })
    }

    pub fn add_projector(&mut self, projector: Box<dyn Projector<Evt, Err> + Send + Sync>) -> &mut Self {
        self.projectors.push(projector);
        self
    }

    async fn persist_and_project(
        &self,
        aggregate_id: Uuid,
        event: Evt,
        sequence_number: SequenceNumber,
    ) -> Result<StoreEvent<Evt>, Err> {
        let store_event: StoreEvent<Evt> = sqlx::query_as::<_, Event>(&self.insert)
            .bind(Uuid::new_v4())
            .bind(aggregate_id)
            .bind(serde_json::to_value(event).unwrap())
            .bind(Utc::now().naive_utc())
            .bind(Utc::now().naive_utc())
            .bind(sequence_number)
            .fetch_one(&self.pool)
            .await?
            .try_into()?;

        self.rebuild_event(&store_event).await?;

        Ok(store_event)
    }
}

#[async_trait]
impl<
        Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
    > EventStore<Evt, Err> for PostgreStore<Evt, Err>
{
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Evt>>, Err> {
        Ok(sqlx::query_as::<_, Event>(&self.select)
            .bind(id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into()?))
            .collect::<Result<Vec<StoreEvent<Evt>>, Err>>()?)
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        event: Evt,
        sequence_number: SequenceNumber,
    ) -> Result<StoreEvent<Evt>, Err> {
        let transaction: Transaction<Postgres> = self.pool.begin().await?;
        match self.persist_and_project(aggregate_id, event, sequence_number).await {
            Ok(event) => {
                transaction.commit().await?;
                Ok(event)
            }
            Err(err) => {
                transaction.rollback().await?;
                Err(err)
            }
        }
    }

    async fn rebuild_event(&self, store_event: &StoreEvent<Evt>) -> Result<(), Err> {
        Ok(for projector in &self.projectors {
            projector.project(store_event).await?
        })
    }
}

#[derive(sqlx::FromRow, Serialize, Deserialize, Debug, Clone)]
struct Event {
    pub id: Uuid,
    pub aggregate_id: Uuid,
    pub payload: Value,
    pub inserted_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub sequence_number: SequenceNumber,
}

impl<E: Serialize + DeserializeOwned + Clone + Send + Sync> TryInto<StoreEvent<E>> for Event {
    type Error = serde_json::Error;

    fn try_into(self) -> Result<StoreEvent<E>, Self::Error> {
        Ok(StoreEvent {
            id: self.id,
            aggregate_id: self.aggregate_id,
            payload: serde_json::from_value::<E>(self.payload)?,
            inserted_at: self.inserted_at,
            updated_at: self.updated_at,
            sequence_number: self.sequence_number,
        })
    }
}

async fn run_preconditions(pool: &Pool<Postgres>, aggregate_name: &str) -> Result<(), sqlx::Error> {
    // Create table if not exists
    let _: PgDone = sqlx::query(create_table(aggregate_name).as_str()).execute(pool).await?;
    // Create 2 indexes if not exist
    let _: PgDone = sqlx::query(create_id_index(aggregate_name).as_str())
        .execute(pool)
        .await?;
    let _: PgDone = sqlx::query(create_aggregate_id_index(aggregate_name).as_str())
        .execute(pool)
        .await?;

    Ok(())
}

fn create_table(aggregate_name: &str) -> String {
    format!(
        "
    CREATE TABLE IF NOT EXISTS {0}_events
    (
      id uuid NOT NULL,
      aggregate_id uuid NOT NULL,
      payload jsonb NOT NULL,
      inserted_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
      updated_at TIMESTAMP NOT NULL DEFAULT current_timestamp,
      sequence_number INT NOT NULL DEFAULT 1,
      CONSTRAINT {0}_events_pkey PRIMARY KEY (id)
    )
    ",
        aggregate_name
    )
}

fn create_id_index(aggregate_name: &str) -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {0}_events_aggregate_id ON public.{0}_events USING btree (((payload ->> 'id'::text)))",
        aggregate_name
    )
}

fn create_aggregate_id_index(aggregate_name: &str) -> String {
    format!(
        "CREATE UNIQUE INDEX IF NOT EXISTS {0}_events_aggregate_id_sequence_number ON {0}_events(aggregate_id, sequence_number)",
        aggregate_name
    )
}

fn select_query(aggregate_name: &str) -> String {
    format!("SELECT * FROM {}_events WHERE aggregate_id = $1", aggregate_name)
}

fn insert_query(aggregate_name: &str) -> String {
    format!(
        "
    INSERT INTO {}_events
    (id, aggregate_id, payload, inserted_at, updated_at, sequence_number)
    VALUES ($1, $2, $3, $4, $5, $6)
    RETURNING *
    ",
        aggregate_name
    )
}
