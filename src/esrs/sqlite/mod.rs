use std::convert::TryInto;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::pool::{PoolConnection, PoolOptions};
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use crate::aggregate::Aggregate;
use policy::SqlitePolicy;
use projector::SqliteProjector;

use crate::esrs::event::Querier;
use crate::esrs::setup::{DatabaseSetup, Setup};
use crate::esrs::sqlite::projector::SqliteProjectorEraser;
use crate::esrs::store::{EraserStore, EventStore, ProjectorStore, StoreEvent};
use crate::esrs::{event, SequenceNumber};

pub mod policy;
pub mod projector;

/// Convenient alias. It needs 4 generics to instantiate `InnerSqliteStore`:
/// - Event
/// - Error
/// - Projector: Default to `dyn SqliteProjector<Event, Error>`
/// - Policy: Default to `dyn SqlitePolicy<Event, Error>`
pub type SqliteStore<
    Event,
    Error,
    Projector = dyn SqliteProjector<Event, Error> + Send + Sync,
    Policy = dyn SqlitePolicy<Event, Error> + Send + Sync,
> = InnerSqliteStore<Event, Error, Projector, Policy>;

/// TODO: some doc here
pub struct InnerSqliteStore<
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error>,
    Projector: SqliteProjector<Event, Error> + Send + Sync + ?Sized,
    Policy: SqlitePolicy<Event, Error> + Send + Sync + ?Sized,
> {
    pool: Pool<Sqlite>,
    projectors: Vec<Box<Projector>>,
    policies: Vec<Box<Policy>>,
    aggregate_name: String,
    phantom_event_type: PhantomData<Event>,
    phantom_error_type: PhantomData<Error>,
    test: bool,
}

impl<
        'a,
        Event: 'a + Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: SqliteProjector<Event, Error> + Send + Sync + ?Sized,
        Policy: SqlitePolicy<Event, Error> + Send + Sync + ?Sized,
    > InnerSqliteStore<Event, Error, Projector, Policy>
{
    /// Prefer this. Pool should be shared between stores
    pub async fn new<T: Aggregate + Sized>(
        pool: &'a Pool<Sqlite>,
        projectors: Vec<Box<Projector>>,
        policies: Vec<Box<Policy>>,
    ) -> Result<Self, Error> {
        let aggregate_name: &str = <T as Aggregate>::name();
        DatabaseSetup::run(aggregate_name, pool).await?;

        Ok(Self {
            pool: pool.clone(),
            projectors,
            policies,
            aggregate_name: aggregate_name.to_string(),
            phantom_event_type: PhantomData::default(),
            phantom_error_type: PhantomData::default(),
            test: false,
        })
    }

    pub async fn test_store<T: Aggregate + Sized>(
        connection_url: &'a str,
        projectors: Vec<Box<Projector>>,
        policies: Vec<Box<Policy>>,
    ) -> Result<Self, Error> {
        let pool: Pool<Sqlite> = PoolOptions::new().max_connections(1).connect(connection_url).await?;
        sqlx::query("BEGIN").execute(&pool).await.map(|_| ())?;

        let aggregate_name: &str = <T as Aggregate>::name();
        DatabaseSetup::run(aggregate_name, &pool).await?;

        Ok(Self {
            pool,
            projectors,
            policies,
            aggregate_name: aggregate_name.to_string(),
            phantom_event_type: PhantomData::default(),
            phantom_error_type: PhantomData::default(),
            test: true,
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

    /// Begin a new transaction. Commit returned transaction or Drop will automatically rollback it
    pub async fn begin(&self) -> Result<PoolConnection<Sqlite>, sqlx::Error> {
        let mut connection = self.pool.acquire().await?;
        let _ = sqlx::query("BEGIN").execute(&mut connection).await?;
        Ok(connection)
    }

    async fn commit(&self, mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
        if !self.test {
            let _ = sqlx::query("COMMIT").execute(&mut connection).await?;
        }
        Ok(())
    }

    async fn rollback(&self, mut connection: PoolConnection<Sqlite>) -> Result<(), sqlx::Error> {
        let _ = sqlx::query("ROLLBACK").execute(&mut connection).await?;
        Ok(())
    }

    pub async fn rebuild_events(&self) -> Result<(), Error> {
        let query: String = event::select_all_query(&self.aggregate_name);
        let mut events = sqlx::query_as::<_, event::Event>(query.as_str()).fetch(&self.pool);

        let mut connection: PoolConnection<Sqlite> = self.begin().await?;

        while let Some(event) = events.try_next().await? {
            let store_event: StoreEvent<Event> = event.try_into()?;
            self.project_event(&store_event, &mut connection).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl<
        Event: Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: SqliteProjector<Event, Error> + Send + Sync + ?Sized,
        Policy: SqlitePolicy<Event, Error> + Send + Sync + ?Sized,
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
        let mut connection: PoolConnection<Sqlite> = self.begin().await?;

        let occurred_on: DateTime<Utc> = Utc::now();

        let events: Vec<_> = events
            .into_iter()
            .map(|e| (Uuid::new_v4(), e))
            .zip(starting_sequence_number..)
            .collect();

        for ((event_id, event), sequence_number) in events.iter() {
            let result: Result<(), Error> = event::Event::insert(
                &self.aggregate_name,
                *event_id,
                aggregate_id,
                event,
                occurred_on,
                *sequence_number,
                &mut *connection,
            )
            .await;

            if let Err(err) = result {
                self.rollback(connection).await?;
                return Err(err);
            }
        }

        let store_events: Vec<_> = events
            .into_iter()
            .map(|((event_id, event), sequence_number)| StoreEvent {
                id: event_id,
                aggregate_id,
                payload: event,
                occurred_on,
                sequence_number,
            })
            .collect();

        for store_event in store_events.iter() {
            let result = self.project_event(store_event, &mut connection).await;

            if let Err(err) = result {
                self.rollback(connection).await?;
                return Err(err);
            }
        }

        self.commit(connection).await?;

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
        Projector: SqliteProjector<Event, Error> + Send + Sync + ?Sized,
        Policy: SqlitePolicy<Event, Error> + Send + Sync + ?Sized,
    > ProjectorStore<Event, PoolConnection<Sqlite>, Error> for InnerSqliteStore<Event, Error, Projector, Policy>
{
    fn project_event<'a>(
        &'a self,
        store_event: &'a StoreEvent<Event>,
        executor: &'a mut PoolConnection<Sqlite>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>
    where
        Self: Sync + 'a,
    {
        async fn run<
            Ev: Serialize + DeserializeOwned + Send + Sync,
            Er: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
            Prj: SqliteProjector<Ev, Er> + Send + Sync + ?Sized,
            Plc: SqlitePolicy<Ev, Er> + Send + Sync + ?Sized,
        >(
            me: &InnerSqliteStore<Ev, Er, Prj, Plc>,
            store_event: &StoreEvent<Ev>,
            executor: &mut PoolConnection<Sqlite>,
        ) -> Result<(), Er> {
            for projector in &me.projectors {
                projector.project(store_event, executor).await?
            }

            Ok(())
        }

        Box::pin(run::<Event, Error, Projector, Policy>(self, store_event, executor))
    }
}

#[async_trait]
impl<
        Event: Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: SqliteProjector<Event, Error> + SqliteProjectorEraser<Event, Error> + Send + Sync + ?Sized,
        Policy: SqlitePolicy<Event, Error> + Send + Sync + ?Sized,
    > EraserStore<Event, Error> for InnerSqliteStore<Event, Error, Projector, Policy>
{
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Error> {
        let mut connection: PoolConnection<Sqlite> = self.begin().await?;
        event::Event::delete_by_aggregate_id::<Event, Error>(&self.aggregate_name, aggregate_id, &self.pool).await?;

        for projector in &self.projectors {
            projector.delete(aggregate_id, &mut connection).await?
        }

        Ok(self.commit(connection).await?)
    }
}
