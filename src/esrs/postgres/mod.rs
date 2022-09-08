use std::convert::TryInto;
use std::future::Future;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::types::Json;
use sqlx::{Executor, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::aggregate::Aggregate;
use crate::esrs::query::Queries;
use crate::esrs::store::{EventStore, StoreEvent};
use crate::esrs::{event, policy, projector, SequenceNumber};

mod setup;

type Projector<Event, Error> = Box<dyn projector::Projector<Postgres, Event, Error> + Send + Sync>;
type Policy<Event, Error> = Box<dyn policy::Policy<Postgres, Event, Error> + Send + Sync>;

pub struct PgStore<Event, Error> {
    pool: Pool<Postgres>,
    queries: Queries,
    projectors: Vec<Projector<Event, Error>>,
    policies: Vec<Policy<Event, Error>>,
}

impl<Event, Error> PgStore<Event, Error>
where
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error>,
{
    pub async fn new<T: Aggregate>(
        pool: &Pool<Postgres>,
        projectors: Vec<Projector<Event, Error>>,
        policies: Vec<Policy<Event, Error>>,
    ) -> Result<Self, Error> {
        setup::run(pool, T::name()).await?;

        Ok(Self {
            pool: pool.clone(),
            queries: Queries::new(T::name()),
            projectors,
            policies,
        })
    }

    pub async fn insert(
        &self,
        aggregate_id: Uuid,
        event: Event,
        occurred_on: &DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<StoreEvent<Event>, Error> {
        let id: Uuid = Uuid::new_v4();

        let _ = sqlx::query(self.queries.insert())
            .bind(id)
            .bind(aggregate_id)
            .bind(Json(&event))
            .bind(occurred_on)
            .bind(sequence_number)
            .execute(executor)
            .await?;

        Ok(StoreEvent {
            id,
            aggregate_id,
            payload: event,
            occurred_on: *occurred_on,
            sequence_number,
        })
    }

    pub async fn delete_by_aggregate_id(
        &self,
        aggregate_id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), Error> {
        Ok(sqlx::query(self.queries.delete())
            .bind(aggregate_id)
            .execute(executor)
            .await
            .map(|_| ())?)
    }

    pub fn projectors(&self) -> &Vec<Projector<Event, Error>> {
        &self.projectors
    }

    pub fn policies(&self) -> &Vec<Policy<Event, Error>> {
        &self.policies
    }

    pub async fn persist_fn<'a, F: Send, T>(&'a self, fun: F) -> Result<Vec<StoreEvent<Event>>, Error>
    where
        F: FnOnce(&'a Pool<Postgres>) -> T,
        T: Future<Output = Result<Vec<StoreEvent<Event>>, Error>> + Send,
    {
        fun(&self.pool).await
    }
}

#[async_trait]
impl<Event, Error> EventStore<Event, Error> for PgStore<Event, Error>
where
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
{
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Event>>, Error> {
        Ok(sqlx::query_as::<_, event::Event>(self.queries.select())
            .bind(id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into()?))
            .collect::<Result<Vec<StoreEvent<Event>>, Error>>()?)
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Event>>, Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;
        let occurred_on: DateTime<Utc> = Utc::now();
        let mut store_events: Vec<StoreEvent<Event>> = vec![];

        for (index, event) in events.into_iter().enumerate() {
            store_events.push(
                self.insert(
                    aggregate_id,
                    event,
                    &occurred_on,
                    starting_sequence_number + index as i32,
                    &mut *transaction,
                )
                .await?,
            )
        }

        for store_event in store_events.iter() {
            for projector in self.projectors() {
                projector.project(store_event, &mut transaction).await?;
            }
        }

        transaction.commit().await?;

        for store_event in store_events.iter() {
            for policy in self.policies() {
                policy.handle_event(store_event, &self.pool).await?;
            }
        }
        Ok(store_events)
    }

    async fn close(&self) {
        self.pool.close().await
    }

    fn get_all(&self) -> BoxStream<Result<StoreEvent<Event>, Error>> {
        Box::pin(
            sqlx::query_as::<_, event::Event>(self.queries.select_all())
                .fetch(&self.pool)
                .map(|res| match res {
                    Ok(e) => match e.try_into() {
                        Ok(e) => Ok(e),
                        Err(e) => Err(Error::from(e)),
                    },
                    Err(e) => Err(e.into()),
                }),
        )
    }
}
