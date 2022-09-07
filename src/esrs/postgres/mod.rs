use std::convert::TryInto;
use std::marker::PhantomData;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::TryStreamExt;
use serde::de::DeserializeOwned;
use serde::Serialize;
use sqlx::types::Json;
use sqlx::{Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::aggregate::Aggregate;
use policy::PgPolicy;
use projector::PgProjector;

use crate::esrs::event;
use crate::esrs::query::Queries;
use crate::esrs::store::{EraserStore, EventStore, ProjectorStore, StoreEvent};
use crate::esrs::SequenceNumber;
use crate::projector::PgProjectorEraser;

mod index;
pub mod policy;
pub mod projector;
mod util;

/// Convenient alias. It needs 4 generics to instantiate `InnerPgStore`:
/// - Event
/// - Error
/// - Projector: Default to `dyn PgProjector<Event, Error>`
/// - Policy: Default to `dyn PgPolicy<Event, Error>`
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
    queries: Queries,
    event: PhantomData<Event>,
    error: PhantomData<Error>,
}

impl<
        Event: Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjector<Event, Error> + Send + Sync + ?Sized,
        Policy: PgPolicy<Event, Error> + Send + Sync + ?Sized,
    > InnerPgStore<Event, Error, Projector, Policy>
{
    /// Prefer this. Pool should be shared between stores
    pub async fn new<T: Aggregate + Sized>(
        pool: &Pool<Postgres>,
        projectors: Vec<Box<Projector>>,
        policies: Vec<Box<Policy>>,
    ) -> Result<Self, Error> {
        // Check if table and indexes exist and possibly create them
        util::run_preconditions(pool, T::name()).await?;

        Ok(Self {
            pool: pool.clone(),
            projectors,
            policies,
            queries: Queries::new(T::name()),
            event: PhantomData::default(),
            error: PhantomData::default(),
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
        let mut events: BoxStream<Result<event::Event, sqlx::Error>> =
            sqlx::query_as::<_, event::Event>(self.queries.select_all()).fetch(&self.pool);

        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        while let Some(event) = events.try_next().await? {
            let store_event: StoreEvent<Event> = event.try_into()?;
            self.project_event(&store_event, &mut transaction).await?;
        }

        Ok(())
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
            let id: Uuid = Uuid::new_v4();
            let sequence_number: SequenceNumber = starting_sequence_number + index as i32;

            let _ = sqlx::query(self.queries.insert())
                .bind(id)
                .bind(aggregate_id)
                .bind(Json(&event))
                .bind(occurred_on)
                .bind(sequence_number)
                .execute(&mut *transaction)
                .await?;

            store_events.push(StoreEvent {
                id,
                aggregate_id,
                payload: event,
                occurred_on,
                sequence_number,
            });
        }

        for store_event in store_events.iter() {
            self.project_event(store_event, &mut transaction).await?;
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

#[async_trait]
impl<
        Event: Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjector<Event, Error> + Send + Sync + ?Sized,
        Policy: PgPolicy<Event, Error> + Send + Sync + ?Sized,
    > ProjectorStore<Event, Transaction<'_, Postgres>, Error> for InnerPgStore<Event, Error, Projector, Policy>
{
    async fn project_event(
        &self,
        store_event: &StoreEvent<Event>,
        executor: &mut Transaction<Postgres>,
    ) -> Result<(), Error> {
        for projector in &self.projectors {
            projector.project(store_event, executor).await?
        }

        Ok(())
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
        let _ = sqlx::query(self.queries.delete())
            .bind(aggregate_id)
            .execute(&mut *transaction)
            .await
            .map(|_| ())?;

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
            _command: Self::Command,
        ) -> Result<Vec<Self::Event>, Self::Error> {
            Ok(vec![])
        }

        fn apply_event(state: Self::State, _payload: Self::Event) -> Self::State {
            state
        }
    }

    #[sqlx::test]
    async fn hello_table_do_not_exist_test(pool: Pool<Postgres>) {
        let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
            .bind(Hello::name())
            .fetch_all(&pool)
            .await
            .unwrap();

        assert!(rows.is_empty());
    }

    #[sqlx::test]
    async fn test_transaction_in_test_store_test(pool: Pool<Postgres>) {
        persist(&pool).await;
        // When
        let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
            .bind(Hello::name())
            .fetch_all(&pool)
            .await
            .unwrap();

        assert!(rows.is_empty());
    }

    async fn persist(pool: &Pool<Postgres>) {
        let aggregate_id: Uuid = Uuid::new_v4();
        let test_store: PgStore<String, Error> = PgStore::new::<Hello>(pool, vec![], vec![]).await.unwrap();
        let _ = test_store
            .persist(aggregate_id, vec!["hello".to_string(), "goodbye".to_string()], 0)
            .await
            .unwrap();
        let list = test_store.by_aggregate_id(aggregate_id).await.unwrap();
        assert_eq!(list.len(), 2);
    }
}
