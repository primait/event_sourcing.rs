use std::convert::TryInto;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::pool::{PoolConnection, PoolOptions};
use sqlx::types::Json;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use policy::PgPolicy;
use projector::PgProjector;

use crate::esrs::aggregate::Identifier;
use crate::esrs::event::Event;
use crate::esrs::query::Queries;
use crate::esrs::store::{EraserStore, EventStore, ProjectorStore, StoreEvent};
use crate::esrs::SequenceNumber;
use crate::projector::PgProjectorEraser;

mod index;
pub mod policy;
pub mod projector;
mod util;

/// Convenient alias. It needs 4 generics to instantiate `InnerSqliteStore`:
/// - Event
/// - Error
/// - Projector: Default to `dyn SqliteProjector<Evt, Err>`
/// - Policy: Default to `dyn SqlitePolicy<Evt, Err>`
pub type PgStore<
    Evt,
    Err,
    Projector = dyn PgProjector<Evt, Err> + Send + Sync,
    Policy = dyn PgPolicy<Evt, Err> + Send + Sync,
> = InnerPgStore<Evt, Err, Projector, Policy>;

/// TODO: some doc here
pub struct InnerPgStore<
    Evt: Serialize + DeserializeOwned + Send + Sync,
    Err: From<sqlx::Error> + From<serde_json::Error>,
    Projector: PgProjector<Evt, Err> + Send + Sync + ?Sized,
    Policy: PgPolicy<Evt, Err> + Send + Sync + ?Sized,
> {
    pool: Pool<Postgres>,
    projectors: Vec<Box<Projector>>,
    policies: Vec<Box<Policy>>,
    queries: Queries,
    evt: PhantomData<Evt>,
    err: PhantomData<Err>,
    test: bool,
}

impl<
        'a,
        Evt: 'a + Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjector<Evt, Err> + Send + Sync + ?Sized,
        Policy: PgPolicy<Evt, Err> + Send + Sync + ?Sized,
    > InnerPgStore<Evt, Err, Projector, Policy>
{
    /// Prefer this. Pool should be shared between stores
    pub async fn new<T: Identifier + Sized>(
        pool: &'a Pool<Postgres>,
        projectors: Vec<Box<Projector>>,
        policies: Vec<Box<Policy>>,
    ) -> Result<Self, Err> {
        let aggregate_name: &str = <T as Identifier>::name();
        // Check if table and indexes exist and possibly create them
        util::run_preconditions(pool, aggregate_name).await?;

        Ok(Self {
            pool: pool.clone(),
            projectors,
            policies,
            queries: Queries::new(aggregate_name),
            evt: PhantomData::default(),
            err: PhantomData::default(),
            test: false,
        })
    }

    pub async fn test<T: Identifier + Sized>(
        connection_url: &'a str,
        projectors: Vec<Box<Projector>>,
        policies: Vec<Box<Policy>>,
    ) -> Result<Self, Err> {
        let pool: Pool<Postgres> = PoolOptions::new().max_connections(1).connect(connection_url).await?;
        sqlx::query("BEGIN").execute(&pool).await.map(|_| ())?;

        let aggregate_name: &str = <T as Identifier>::name();
        // Check if table and indexes exist and eventually create them
        util::run_preconditions(&pool, aggregate_name).await?;

        Ok(Self {
            pool,
            projectors,
            policies,
            queries: Queries::new(aggregate_name),
            evt: PhantomData::default(),
            err: PhantomData::default(),
            test: true,
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

    /// Begin a new transaction. Commit returned transaction or Drop will automatically rollback it
    pub async fn begin(&self) -> Result<PoolConnection<Postgres>, sqlx::Error> {
        let mut connection = self.pool.acquire().await?;
        let _ = sqlx::query("BEGIN").execute(&mut connection).await?;
        Ok(connection)
    }

    async fn commit(&self, mut connection: PoolConnection<Postgres>) -> Result<(), sqlx::Error> {
        if !self.test {
            let _ = sqlx::query("COMMIT").execute(&mut connection).await?;
        }
        Ok(())
    }

    async fn rollback(&self, mut connection: PoolConnection<Postgres>) -> Result<(), sqlx::Error> {
        let _ = sqlx::query("ROLLBACK").execute(&mut connection).await?;
        Ok(())
    }

    pub async fn rebuild_events(&self) -> Result<(), Err> {
        let mut events: BoxStream<Result<Event, sqlx::Error>> =
            sqlx::query_as::<_, Event>(self.queries.select_all()).fetch(&self.pool);

        let mut connection: PoolConnection<Postgres> = self.begin().await?;

        while let Some(event) = events.try_next().await? {
            let evt: StoreEvent<Evt> = event.try_into()?;
            self.project_event(&evt, &mut connection).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl<
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjector<Evt, Err> + Send + Sync + ?Sized,
        Policy: PgPolicy<Evt, Err> + Send + Sync + ?Sized,
    > EventStore<Evt, Err> for InnerPgStore<Evt, Err, Projector, Policy>
{
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Evt>>, Err> {
        Ok(sqlx::query_as::<_, Event>(self.queries.select())
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
        events: Vec<Evt>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Evt>>, Err> {
        let mut connection: PoolConnection<Postgres> = self.begin().await?;

        let occurred_on: DateTime<Utc> = Utc::now();

        let events: Vec<_> = events
            .into_iter()
            .map(|e| (Uuid::new_v4(), e))
            .zip(starting_sequence_number..)
            .collect();

        for ((event_id, event), sequence_number) in events.iter() {
            let result = sqlx::query(self.queries.insert())
                .bind(event_id)
                .bind(aggregate_id)
                .bind(Json(event))
                .bind(occurred_on)
                .bind(sequence_number)
                .execute(&mut *connection)
                .await;

            if let Err(err) = result {
                self.rollback(connection).await?;
                return Err(err.into());
            }
        }

        let mut store_events: Vec<StoreEvent<Evt>> = vec![];

        for ((event_id, event), sequence_number) in events {
            let store_event: StoreEvent<Evt> = StoreEvent {
                id: event_id,
                aggregate_id,
                payload: event,
                occurred_on,
                sequence_number,
            };

            match self.project_event(&store_event, &mut connection).await {
                Ok(()) => store_events.push(store_event),
                Err(err) => {
                    self.rollback(connection).await?;
                    return Err(err);
                }
            }
        }

        self.commit(connection).await?;

        Ok(store_events)
    }

    /// Default `run_policies` strategy is to run all events against each policy in turn, returning on the first error.
    async fn run_policies(&self, events: &[StoreEvent<Evt>]) -> Result<(), Err> {
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
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjector<Evt, Err> + Send + Sync + ?Sized,
        Policy: PgPolicy<Evt, Err> + Send + Sync + ?Sized,
    > ProjectorStore<Evt, PoolConnection<Postgres>, Err> for InnerPgStore<Evt, Err, Projector, Policy>
{
    fn project_event<'a>(
        &'a self,
        store_event: &'a StoreEvent<Evt>,
        executor: &'a mut PoolConnection<Postgres>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
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
            executor: &mut PoolConnection<Postgres>,
        ) -> Result<(), Er> {
            for projector in &me.projectors {
                projector.project(store_event, executor).await?
            }

            Ok(())
        }

        Box::pin(run::<Evt, Err, Projector, Policy>(self, store_event, executor))
    }
}

#[async_trait]
impl<
        Evt: Serialize + DeserializeOwned + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjectorEraser<Evt, Err> + Send + Sync + ?Sized,
        Policy: PgPolicy<Evt, Err> + Send + Sync + ?Sized,
    > EraserStore<Evt, Err> for InnerPgStore<Evt, Err, Projector, Policy>
{
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Err> {
        let mut connection: PoolConnection<Postgres> = self.begin().await?;
        let _ = sqlx::query(self.queries.delete())
            .bind(aggregate_id)
            .execute(&mut *connection)
            .await
            .map(|_| ());

        for projector in &self.projectors {
            projector.delete(aggregate_id, &mut connection).await?
        }

        Ok(self.commit(connection).await?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    impl Identifier for Hello {
        fn name() -> &'static str
        where
            Self: Sized,
        {
            "hello"
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
