use std::convert::TryInto;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::aggregate::Aggregate;
use policy::PgPolicy;
use projector::PgProjector;

use crate::esrs::event::Querier;
use crate::esrs::setup::{DatabaseSetup, Setup};
use crate::esrs::store::{EraserStore, EventStore, ProjectorStore, StoreEvent};
use crate::esrs::{event, SequenceNumber};
use crate::projector::PgProjectorEraser;

pub mod policy;
pub mod projector;

/// Convenient alias. It needs 4 generics to instantiate `InnerSqliteStore`:
/// - Event
/// - Error
/// - Projector: Default to `dyn SqliteProjector<Event, Error>`
/// - Policy: Default to `dyn SqlitePolicy<Event, Error>`
pub type PgStore<
    Event,
    Error,
    Projector = dyn PgProjector<Event, Error> + Send + Sync,
    Policy = dyn PgPolicy<Event, Error> + Send + Sync,
> = InnerPgStore<Event, Error, Projector, Policy>;

/// TODO: some doc here
pub struct InnerPgStore<
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error>,
    Projector: PgProjector<Event, Error> + Send + Sync + ?Sized,
    Policy: PgPolicy<Event, Error> + Send + Sync + ?Sized,
> {
    pool: Pool<Postgres>,
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
        Projector: PgProjector<Event, Error> + Send + Sync + ?Sized,
        Policy: PgPolicy<Event, Error> + Send + Sync + ?Sized,
    > InnerPgStore<Event, Error, Projector, Policy>
{
    /// Prefer this. Pool should be shared between stores
    pub async fn new<T: Aggregate + Sized>(
        pool: &'a Pool<Postgres>,
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
        })
    }

    pub const fn pool(&self) -> &Pool<Postgres> {
        &self.pool
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

        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

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
        Projector: PgProjector<Event, Error> + Send + Sync + ?Sized,
        Policy: PgPolicy<Event, Error> + Send + Sync + ?Sized,
    > EventStore<Event, Error> for InnerPgStore<Event, Error, Projector, Policy>
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
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

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
        Projector: PgProjector<Event, Error> + Send + Sync + ?Sized,
        Policy: PgPolicy<Event, Error> + Send + Sync + ?Sized,
    > ProjectorStore<Event, Transaction<'_, Postgres>, Error> for InnerPgStore<Event, Error, Projector, Policy>
{
    fn project_event<'a>(
        &'a self,
        store_event: &'a StoreEvent<Event>,
        executor: &'a mut Transaction<Postgres>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Error>> + Send + 'a>>
    where
        Self: Sync + 'a,
    {
        async fn run<
            Ev: Serialize + DeserializeOwned + Send + Sync,
            Er: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
            Prj: PgProjector<Ev, Er> + Send + Sync + ?Sized,
            Plc: PgPolicy<Ev, Er> + Send + Sync + ?Sized,
        >(
            me: &InnerPgStore<Ev, Er, Prj, Plc>,
            store_event: &StoreEvent<Ev>,
            executor: &mut Transaction<'_, Postgres>,
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
        Projector: PgProjectorEraser<Event, Error> + Send + Sync + ?Sized,
        Policy: PgPolicy<Event, Error> + Send + Sync + ?Sized,
    > EraserStore<Event, Error> for InnerPgStore<Event, Error, Projector, Policy>
{
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        // TODO: from pool to transaction
        event::Event::delete_by_aggregate_id::<Event, Error>(&self.aggregate_name, aggregate_id, &self.pool).await?;

        for projector in &self.projectors {
            projector.delete(aggregate_id, &mut transaction).await?
        }

        Ok(transaction.commit().await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::aggregate::AggregateState;

    #[derive(Debug)]
    pub enum Error {
        Json,
        Sql,
    }

    impl From<serde_json::Error> for Error {
        fn from(_: serde_json::Error) -> Self {
            Self::Json
        }
    }

    impl From<sqlx::Error> for Error {
        fn from(_: sqlx::Error) -> Self {
            Self::Sql
        }
    }

    struct Hello;
    impl Aggregate for Hello {
        type State = ();
        type Command = ();
        type Event = ();
        type Error = ();

        fn name() -> &'static str
        where
            Self: Sized,
        {
            "hello"
        }

        fn handle_command(
            _state: &AggregateState<Self::State>,
            _cmd: Self::Command,
        ) -> Result<Vec<Self::Event>, Self::Error> {
            Ok(vec![])
        }

        fn apply_event(state: Self::State, _payload: &Self::Event) -> Self::State {
            state
        }
    }

    #[tokio::test]
    async fn hello_table_do_not_exist_test() {
        let database_url = std::env::var("DATABASE_URL").unwrap();
        let pool: Pool<Postgres> = PoolOptions::new().connect(database_url.as_str()).await.unwrap();
        let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
            .bind(Hello::name())
            .fetch_all(&pool)
            .await
            .unwrap();

        assert!(rows.is_empty());
    }

    #[tokio::test]
    async fn test_transaction_in_test_store_test() {
        let database_url = std::env::var("DATABASE_URL").unwrap();
        persist(database_url.as_str()).await;
        let pool: Pool<Postgres> = PoolOptions::new().connect(database_url.as_str()).await.unwrap();
        // When
        let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
            .bind(Hello::name())
            .fetch_all(&pool)
            .await
            .unwrap();

        assert!(rows.is_empty());
    }

    async fn persist(database_url: &str) {
        let aggregate_id: Uuid = Uuid::new_v4();
        let test_store: PgStore<String, Error> = PgStore::test::<Hello>(database_url, vec![], vec![]).await.unwrap();
        let _ = test_store
            .persist(aggregate_id, vec!["hello".to_string(), "goodbye".to_string()], 0)
            .await
            .unwrap();
        let list = test_store.by_aggregate_id(aggregate_id).await.unwrap();
        assert_eq!(list.len(), 2);
    }
}
