use std::convert::TryInto;
use std::future::Future;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::StreamExt;
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{Executor, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::esrs::policy;
use crate::types::SequenceNumber;
use crate::{AggregateManager, EventStore, StoreEvent};

use super::{event, projector, statement::Statements};

type Projector<A> = Box<dyn projector::Projector<A> + Send + Sync>;
type Policy<A> = Box<dyn policy::Policy<A> + Send + Sync>;

#[derive(Clone)]
pub struct PgStore<Manager>
where
    Manager: AggregateManager,
{
    pool: Pool<Postgres>,
    statements: Statements,
    projectors: Vec<Projector<Manager>>,
    policies: Vec<Policy<Manager>>,
}

impl<Manager> PgStore<Manager>
where
    Manager: AggregateManager,
    Manager::State: Default + Clone + Send + Sync,
    Manager::Command: Send,
    Manager::Event: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
    Manager::Error: From<sqlx::Error> + From<serde_json::Error>,
{
    /// Creates a new implementation of an aggregate
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self {
            pool,
            statements: Statements::new(Manager::name()),
            projectors: vec![],
            policies: vec![],
        }
    }

    /// Append a projector to projectors list
    pub fn add_projector(&mut self, projector: Projector<Manager>) {
        self.projectors.push(projector);
    }

    /// Set the list of projectors to the store
    pub fn set_projectors(self, projectors: Vec<Projector<Manager>>) -> Self {
        Self { projectors, ..self }
    }

    /// Append a policy to policies list
    pub fn add_policy(&mut self, policy: Policy<Manager>) {
        self.policies.push(policy);
    }

    /// Set the list of policies to the store
    pub fn set_policies(self, policies: Vec<Policy<Manager>>) -> Self {
        Self { policies, ..self }
    }

    /// This function setup the database in a transaction, creating the event store table (if not exists)
    /// and two indexes (always if not exist). The first one is over the `aggregate_id` field to
    /// speed up `by_aggregate_id` query. The second one is a unique constraint over the tuple
    /// `(aggregate_id, sequence_number)` to avoid race conditions.
    ///
    /// This function should be used only once at your application startup. It tries to create the
    /// event table and its indexes if they not exist.
    pub async fn setup(self) -> Result<Self, Manager::Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        // Create events table if not exists
        let _: PgQueryResult = sqlx::query(self.statements.create_table())
            .execute(&mut *transaction)
            .await?;

        // Create index on aggregate_id for `by_aggregate_id` query.
        let _: PgQueryResult = sqlx::query(self.statements.create_index())
            .execute(&mut *transaction)
            .await?;

        // Create unique constraint `aggregate_id`-`sequence_number` to avoid race conditions.
        let _: PgQueryResult = sqlx::query(self.statements.create_unique_constraint())
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(self)
    }

    /// Save an event in the event store and return a new `StoreEvent` instance.
    pub async fn save_event(
        &self,
        aggregate_id: Uuid,
        event: Manager::Event,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<StoreEvent<Manager::Event>, Manager::Error> {
        let id: Uuid = Uuid::new_v4();

        let _ = sqlx::query(self.statements.insert())
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
            occurred_on,
            sequence_number,
        })
    }

    /// This function returns a stream representing the full event store table content. This should
    /// be mainly used to rebuild read models.
    pub fn get_all(&self) -> BoxStream<Result<StoreEvent<Manager::Event>, Manager::Error>> {
        Box::pin({
            sqlx::query_as::<_, event::Event>(self.statements.select_all())
                .fetch(&self.pool)
                .map(|res| match res {
                    Ok(event) => event.try_into().map_err(Into::into),
                    Err(error) => Err(error.into()),
                })
        })
    }

    /// This function returns the list of all projections added to this store. This function should
    /// mostly used while creating a custom persistence flow using [`PgStore::persist`].
    pub fn projectors(&self) -> &[Projector<Manager>] {
        &self.projectors
    }

    /// This function returns the list of all policies added to this store. This function should
    /// mostly used while creating a custom persistence flow using [`PgStore::persist`].
    pub fn policies(&self) -> &[Policy<Manager>] {
        &self.policies
    }

    /// This function could be used in order to customize the way the store persist the events.
    /// For example could be used to avoid having projectors in transaction with event saving. Or to
    /// let the policies return or not an error if one of them fails.
    ///
    /// An example of how to use this function is in `examples/customize_persistence_flow` example
    /// folder.
    pub async fn persist<'a, F: Send, T>(&'a self, fun: F) -> Result<Vec<StoreEvent<Manager::Event>>, Manager::Error>
    where
        F: FnOnce(&'a Pool<Postgres>) -> T,
        T: Future<Output = Result<Vec<StoreEvent<Manager::Event>>, Manager::Error>> + Send,
    {
        fun(&self.pool).await
    }

    /// This function closes the inner pool.
    pub async fn close(&self) {
        self.pool.close().await
    }
}

#[async_trait]
impl<Manager> EventStore for PgStore<Manager>
where
    Manager: AggregateManager,
    Manager::State: Default + Clone + Send + Sync,
    Manager::Command: Send,
    Manager::Event: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
    Manager::Error: From<sqlx::Error> + From<serde_json::Error>,
{
    type Manager = Manager;

    async fn by_aggregate_id(&self, aggregate_id: Uuid) -> Result<Vec<StoreEvent<Manager::Event>>, Manager::Error> {
        Ok(sqlx::query_as::<_, event::Event>(self.statements.by_aggregate_id())
            .bind(aggregate_id)
            .fetch_all(&self.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into()?))
            .collect::<Result<Vec<StoreEvent<Manager::Event>>, Manager::Error>>()?)
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<Manager::Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Manager::Event>>, Manager::Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;
        let occurred_on: DateTime<Utc> = Utc::now();
        let mut store_events: Vec<StoreEvent<Manager::Event>> = vec![];

        for (index, event) in events.into_iter().enumerate() {
            store_events.push(
                self.save_event(
                    aggregate_id,
                    event,
                    occurred_on,
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
                let _ = policy.handle_event(store_event).await;
            }
        }

        Ok(store_events)
    }

    async fn delete_by_aggregate_id(&self, aggregate_id: Uuid) -> Result<(), Manager::Error> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        let _ = sqlx::query(self.statements.delete_by_aggregate_id())
            .bind(aggregate_id)
            .execute(&mut *transaction)
            .await
            .map(|_| ())?;

        for projector in &self.projectors {
            projector.delete(aggregate_id, &mut *transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}
