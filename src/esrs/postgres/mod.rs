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

use crate::esrs::aggregate::Aggregate;
use crate::esrs::event::Querier;
use crate::esrs::setup::{DatabaseSetup, Setup};
use crate::esrs::store::{EraserStore, EventStore, ProjectorStore, StoreEvent};
use crate::esrs::{event, policy, projector, SequenceNumber};

/// Convenient alias. It needs 4 generics to instantiate `InnerSqliteStore`:
/// - Event
/// - Error
/// - Projector: Default to `dyn Projector<Transaction<'static, Postgres>, Event, Error>`
/// - Policy: Default to `dyn Policy<Pool<Postgres>, Event, Error>`
pub type PgStore<
    Event,
    Error,
    Projector = dyn projector::Projector<Transaction<'static, Postgres>, Event, Error> + Send + Sync,
    Policy = dyn policy::Policy<Pool<Postgres>, Event, Error> + Send + Sync,
> = InnerPgStore<Event, Error, Projector, Policy>;

/// TODO: some doc here
pub struct InnerPgStore<
    Event: Serialize + DeserializeOwned + Send + Sync,
    Error: From<sqlx::Error> + From<serde_json::Error>,
    Projector: projector::Projector<Transaction<'static, Postgres>, Event, Error> + Send + Sync + ?Sized,
    Policy: policy::Policy<Pool<Postgres>, Event, Error> + Send + Sync + ?Sized,
> {
    pool: Pool<Postgres>,
    projectors: Vec<Box<Projector>>,
    policies: Vec<Box<Policy>>,
    aggregate_name: String,
    phantom_event_type: PhantomData<Event>,
    phantom_error_type: PhantomData<Error>,
}

impl<
        'e,
        Event: 'e + Serialize + DeserializeOwned + Send + Sync,
        Error: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: projector::Projector<Transaction<'static, Postgres>, Event, Error> + Send + Sync + ?Sized,
        Policy: policy::Policy<Pool<Postgres>, Event, Error> + Send + Sync + ?Sized,
    > InnerPgStore<Event, Error, Projector, Policy>
{
    pub async fn new<T: Aggregate + Sized>(
        pool: &'e Pool<Postgres>,
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
        Projector: projector::Projector<Transaction<'static, Postgres>, Event, Error> + Send + Sync + ?Sized,
        Policy: policy::Policy<Pool<Postgres>, Event, Error> + Send + Sync + ?Sized,
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
        Projector: projector::Projector<Transaction<'static, Postgres>, Event, Error> + Send + Sync + ?Sized,
        Policy: policy::Policy<Pool<Postgres>, Event, Error> + Send + Sync + ?Sized,
    > ProjectorStore<Transaction<'static, Postgres>, Event, Error> for InnerPgStore<Event, Error, Projector, Policy>
{
    fn project_event<'a>(
        &'a self,
        store_event: &'a StoreEvent<Event>,
        executor: &'a mut Transaction<'static, Postgres>,
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
        Projector: projector::ProjectorEraser<Transaction<'static, Postgres>, Event, Error> + Send + Sync + ?Sized,
        Policy: policy::Policy<Pool<Postgres>, Event, Error> + Send + Sync + ?Sized,
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
    use crate::aggregate::AggregateState;

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
    async fn test_transaction_in_test_store_test(pool: Pool<Postgres>) -> sqlx::Result<()> {
        persist(&pool).await;
        // When
        let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
            .bind(Hello::name())
            .fetch_all(&pool)
            .await
            .unwrap();

        assert!(rows.is_empty());
        Ok(())
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
