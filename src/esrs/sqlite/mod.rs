use std::convert::TryInto;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Sqlite, Transaction};
use uuid::Uuid;

use crate::esrs::aggregate::Aggregate;
use crate::esrs::event::Querier;
use crate::esrs::setup::{DatabaseSetup, Setup};
use crate::esrs::store::{EraserStore, EventStore, ProjectorStore, StoreEvent};
use crate::esrs::{event, policy, projector, SequenceNumber};

/// Convenient alias. It needs 4 generics to instantiate `InnerSqliteStore`:
/// - Event
/// - Error
/// - Projector: Default to `dyn projector::Projector<Transaction<'static, Sqlite>, Event, Error>`
/// - Policy: Default to `dyn policy::Policy<Pool<Sqlite>, Event, Error>`
pub type SqliteStore<
    Event,
    Error,
    Projector = dyn projector::Projector<Transaction<'static, Sqlite>, Event, Error> + Send + Sync,
    Policy = dyn policy::Policy<Pool<Sqlite>, Event, Error> + Send + Sync,
> = InnerSqliteStore<Event, Error, Projector, Policy>;

/// TODO: some doc here
pub struct InnerSqliteStore<
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error>,
    Projector: projector::Projector<Transaction<'static, Sqlite>, Event, Error> + Send + Sync + ?Sized,
    Policy: policy::Policy<Pool<Sqlite>, Event, Error> + Send + Sync + ?Sized,
> {
    pool: Pool<Sqlite>,
    projectors: Vec<Box<Projector>>,
    policies: Vec<Box<Policy>>,
    aggregate_name: String,
    phantom_event_type: PhantomData<Event>,
    phantom_error_type: PhantomData<Error>,
}

impl<
        'a,
        Event: 'a + Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: projector::Projector<Transaction<'static, Sqlite>, Event, Error> + Send + Sync + ?Sized,
        Policy: policy::Policy<Pool<Sqlite>, Event, Error> + Send + Sync + ?Sized,
    > InnerSqliteStore<Event, Error, Projector, Policy>
{
    pub async fn new<T: Aggregate + Sized>(
        pool: &'a Pool<Sqlite>,
        projectors: Vec<Box<Projector>>,
        policies: Vec<Box<Policy>>,
    ) -> Result<Self, Error> {
        DatabaseSetup::run(T::name(), pool).await?;

        Ok(Self {
            pool: pool.clone(),
            projectors,
            policies,
            aggregate_name: T::name().to_string(),
            phantom_event_type: PhantomData::default(),
            phantom_error_type: PhantomData::default(),
        })
    }

    pub fn add_projector(&mut self, projector: Box<Projector>) -> &mut Self {
        self.projectors.push(projector);
        self
    }

    pub fn add_policy(&mut self, policy: Box<Policy>) -> &mut Self {
        self.policies.push(policy);
        self
    }

    pub async fn rebuild_events(&self) -> Result<(), Error> {
        let query: String = event::select_all_query(&self.aggregate_name);
        let mut events = sqlx::query_as::<_, event::Event>(query.as_str()).fetch(&self.pool);

        let mut transaction: Transaction<Sqlite> = self.pool.begin().await?;

        while let Some(event) = events.try_next().await? {
            let store_event: StoreEvent<Event> = event.try_into()?;
            self.project_event(&store_event, &mut transaction).await?;
        }

        Ok(transaction.commit().await?)
    }
}

#[async_trait]
impl<
        Event: Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: projector::Projector<Transaction<'static, Sqlite>, Event, Error> + Send + Sync + ?Sized,
        Policy: policy::Policy<Pool<Sqlite>, Event, Error> + Send + Sync + ?Sized,
    > EventStore<Event, Error> for InnerSqliteStore<Event, Error, Projector, Policy>
{
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Event>>, Error> {
        event::Event::by_aggregate_id(&self.aggregate_name, id, &self.pool).await
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Event>>, Error> {
        let mut transaction: Transaction<Sqlite> = self.pool.begin().await?;

        let occurred_on: DateTime<Utc> = Utc::now();

        let events: Vec<_> = events
            .into_iter()
            .map(|e| (Uuid::new_v4(), e))
            .zip(starting_sequence_number..)
            .collect();

        for ((event_id, event), sequence_number) in events.iter() {
            event::Event::insert::<Event, Error>(
                &self.aggregate_name,
                *event_id,
                aggregate_id,
                event,
                occurred_on,
                *sequence_number,
                &mut *transaction,
            )
            .await?;
        }

        let mut store_events: Vec<StoreEvent<Event>> = vec![];

        for ((event_id, event), sequence_number) in events {
            let store_event: StoreEvent<Event> = StoreEvent {
                id: event_id,
                aggregate_id,
                payload: event,
                occurred_on,
                sequence_number,
            };

            self.project_event(&store_event, &mut transaction).await?;
            store_events.push(store_event)
        }

        transaction.commit().await?;
        Ok(store_events)
    }

    /// Default `run_policies` strategy is to run all events against each policy in turn, returning on the first error.
    async fn run_policies(&self, events: &[StoreEvent<Event>]) -> Result<(), Error> {
        // TODO: This implies that potentially half of the policies would trigger, then one fails, and the rest wouldn't.
        // potentially we should be returning some other kind of error, that includes the errors from any failed policies?
        for policy in &self.policies {
            for event in events.iter() {
                policy.handle_event(event, &self.pool).await?
            }
        }

        Ok(())
    }

    async fn close(&self) {
        self.pool.close().await
    }
}

impl<
        Event: Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: projector::Projector<Transaction<'static, Sqlite>, Event, Error> + Send + Sync + ?Sized,
        Policy: policy::Policy<Pool<Sqlite>, Event, Error> + Send + Sync + ?Sized,
    > ProjectorStore<Transaction<'static, Sqlite>, Event, Error> for InnerSqliteStore<Event, Error, Projector, Policy>
{
    fn project_event<'a>(
        &'a self,
        store_event: &'a StoreEvent<Event>,
        executor: &'a mut Transaction<'static, Sqlite>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>
    where
        Self: Sync + 'a,
    {
        Box::pin(async move {
            for projector in &self.projectors {
                projector.project(store_event, executor).await?
            }

            Ok(())
        })
    }
}

#[async_trait]
impl<
        Event: Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: projector::ProjectorEraser<Transaction<'static, Sqlite>, Event, Error> + Send + Sync + ?Sized,
        Policy: policy::Policy<Pool<Sqlite>, Event, Error> + Send + Sync + ?Sized,
    > EraserStore<Event, Error> for InnerSqliteStore<Event, Error, Projector, Policy>
{
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Error> {
        let mut transaction: Transaction<Sqlite> = self.pool.begin().await?;

        // TODO: use transaction instead of pool
        event::Event::delete_by_aggregate_id::<Event, Error>(&self.aggregate_name, aggregate_id, &self.pool).await?;

        for projector in &self.projectors {
            projector.delete(aggregate_id, &mut transaction).await?
        }

        Ok(transaction.commit().await?)
    }
}
