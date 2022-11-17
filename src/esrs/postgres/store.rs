use std::convert::TryInto;
use std::future::Future;
use std::sync::Arc;

use arc_swap::ArcSwap;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::StreamExt;
use sqlx::postgres::PgQueryResult;
use sqlx::types::Json;
use sqlx::{Executor, Pool, Postgres, Transaction};
use tracing::Instrument;
use uuid::Uuid;

use crate::esrs::policy;
use crate::esrs::postgres::projector::Consistency;
use crate::types::SequenceNumber;
use crate::{Aggregate, AggregateManager, EventStore, StoreEvent};

use super::{event, projector, statement::Statements};

type Projector<A> = Box<dyn projector::Projector<A> + Send + Sync>;
type Policy<A> = Box<dyn policy::Policy<A> + Send + Sync>;

/// Default Postgres implementation for the [`EventStore`]. Use this struct in order to have a
/// pre-made implementation of an [`EventStore`] persisting on Postgres.
///
/// The store is protected by an [`Arc`] that allows it to be cloneable still having the same memory
/// reference.
#[derive(Clone)]
pub struct PgStore<Manager>
where
    Manager: AggregateManager,
{
    inner: Arc<InnerPgStore<Manager>>,
}

pub struct InnerPgStore<Manager>
where
    Manager: AggregateManager,
{
    pool: Pool<Postgres>,
    statements: Statements,
    projectors: ArcSwap<Vec<Projector<Manager>>>,
    policies: ArcSwap<Vec<Policy<Manager>>>,
}

impl<Manager> PgStore<Manager>
where
    Manager: AggregateManager,
    Manager::State: Default + Clone + Send + Sync,
    Manager::Command: Send,
    Manager::Event: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
    Manager::Error: From<sqlx::Error> + From<serde_json::Error> + std::error::Error,
{
    /// Creates a new implementation of an aggregate
    #[must_use]
    pub fn new(pool: Pool<Postgres>) -> Self {
        let inner: InnerPgStore<Manager> = InnerPgStore {
            pool,
            statements: Statements::new(Manager::name()),
            projectors: ArcSwap::from_pointee(vec![]),
            policies: ArcSwap::from_pointee(vec![]),
        };

        Self { inner: Arc::new(inner) }
    }

    /// Set the list of projectors to the store
    pub fn set_projectors(self, projectors: Vec<Projector<Manager>>) -> Self {
        self.inner.projectors.store(Arc::new(projectors));
        self
    }

    /// Set the list of policies to the store
    pub fn set_policies(self, policies: Vec<Policy<Manager>>) -> Self {
        self.inner.policies.store(Arc::new(policies));
        self
    }

    /// This function setup the database in a transaction, creating the event store table (if not exists)
    /// and two indexes (always if not exist). The first one is over the `aggregate_id` field to
    /// speed up `by_aggregate_id` query. The second one is a unique constraint over the tuple
    /// `(aggregate_id, sequence_number)` to avoid race conditions.
    ///
    /// This function should be used only once at your application startup. It tries to create the
    /// event table and its indexes if they not exist.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if there's an error connecting with database or creating tables/indexes.
    pub async fn setup(self) -> Result<Self, Manager::Error> {
        let mut transaction: Transaction<Postgres> = self.inner.pool.begin().await?;

        // Create events table if not exists
        let _: PgQueryResult = sqlx::query(self.inner.statements.create_table())
            .execute(&mut *transaction)
            .await?;

        // Create index on aggregate_id for `by_aggregate_id` query.
        let _: PgQueryResult = sqlx::query(self.inner.statements.create_index())
            .execute(&mut *transaction)
            .await?;

        // Create unique constraint `aggregate_id`-`sequence_number` to avoid race conditions.
        let _: PgQueryResult = sqlx::query(self.inner.statements.create_unique_constraint())
            .execute(&mut *transaction)
            .await?;

        transaction.commit().await?;

        Ok(self)
    }

    /// Save an event in the event store and return a new `StoreEvent` instance.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the insert of the values into the database fails.
    pub async fn save_event(
        &self,
        aggregate_id: Uuid,
        event: Manager::Event,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<StoreEvent<Manager::Event>, Manager::Error> {
        let id: Uuid = Uuid::new_v4();

        let _ = sqlx::query(self.inner.statements.insert())
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
    pub fn stream_events<'s>(
        &'s self,
        executor: impl Executor<'s, Database = Postgres> + 's,
    ) -> BoxStream<Result<StoreEvent<Manager::Event>, Manager::Error>> {
        Box::pin({
            sqlx::query_as::<_, event::Event>(self.inner.statements.select_all())
                .fetch(executor)
                .map(|res| Ok(res?.try_into()?))
        })
    }

    /// This function returns the list of all projections added to this store. This function should
    /// mostly used while creating a custom persistence flow using [`PgStore::persist`].
    pub fn projectors(&self) -> Arc<Vec<Projector<Manager>>> {
        self.inner.projectors.load().clone()
    }

    /// This function returns the list of all policies added to this store. This function should
    /// mostly used while creating a custom persistence flow using [`PgStore::persist`].
    pub fn policies(&self) -> Arc<Vec<Policy<Manager>>> {
        self.inner.policies.load().clone()
    }

    /// This function could be used in order to customize the way the store persist the events.
    /// For example could be used to avoid having projectors in transaction with event saving. Or to
    /// let the policies return or not an error if one of them fails.
    ///
    /// An example of how to use this function is in `examples/customize_persistence_flow` example
    /// folder.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the given `fun` returns an `Err`. In the `EventStore` implementation
    /// for `PgStore` this function return an `Err` if the event insertion or its projection fails.
    pub async fn persist<'a, F, T>(&'a self, fun: F) -> Result<Vec<StoreEvent<Manager::Event>>, Manager::Error>
    where
        F: Send + FnOnce(&'a Pool<Postgres>) -> T,
        T: Future<Output = Result<Vec<StoreEvent<Manager::Event>>, Manager::Error>> + Send,
    {
        fun(&self.inner.pool).await
    }
}

#[async_trait]
impl<Manager> EventStore for PgStore<Manager>
where
    Manager: AggregateManager,
    Manager::State: Default + Clone + Send + Sync,
    Manager::Command: Send,
    Manager::Event: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
    Manager::Error: From<sqlx::Error> + From<serde_json::Error> + std::error::Error,
{
    type Manager = Manager;

    async fn by_aggregate_id(&self, aggregate_id: Uuid) -> Result<Vec<StoreEvent<Manager::Event>>, Manager::Error> {
        Ok(
            sqlx::query_as::<_, event::Event>(self.inner.statements.by_aggregate_id())
                .bind(aggregate_id)
                .fetch_all(&self.inner.pool)
                .await?
                .into_iter()
                .map(|event| Ok(event.try_into()?))
                .collect::<Result<Vec<StoreEvent<Manager::Event>>, Manager::Error>>()?,
        )
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        events: Vec<Manager::Event>,
        starting_sequence_number: SequenceNumber,
    ) -> Result<Vec<StoreEvent<Manager::Event>>, Manager::Error> {
        let mut transaction: Transaction<Postgres> = self.inner.pool.begin().await?;
        let occurred_on: DateTime<Utc> = Utc::now();
        let mut store_events: Vec<StoreEvent<Manager::Event>> = vec![];

        for (index, event) in (0..).zip(events.into_iter()) {
            let store_event: StoreEvent<<Manager as Aggregate>::Event> = self
                .save_event(
                    aggregate_id,
                    event,
                    occurred_on,
                    starting_sequence_number + index,
                    &mut *transaction,
                )
                .await?;

            store_events.push(store_event);
        }

        for store_event in &store_events {
            for projector in self.projectors().iter() {
                let span = tracing::trace_span!(
                    "esrs_project_event",
                    event_id = %store_event.id,
                    aggregate_id = %store_event.aggregate_id,
                    consistency = projector.consistency().as_ref(),
                    projector = projector.name()
                );
                match projector.consistency() {
                    Consistency::Strong => {
                        projector
                            .project(store_event, &mut transaction)
                            .instrument(span)
                            .await?
                    }
                    Consistency::Eventual => {
                        let _result = projector.project(store_event, &mut transaction).instrument(span).await;
                    }
                }
            }
        }

        transaction.commit().await?;

        for store_event in &store_events {
            for policy in self.policies().iter() {
                let span = tracing::info_span!("esrs_apply_policy" , event_id = %store_event.id, aggregate_id = %store_event.aggregate_id, policy = policy.name());
                let _policy_result = policy.handle_event(store_event).instrument(span).await;
            }
        }

        Ok(store_events)
    }

    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Manager::Error> {
        let mut transaction: Transaction<Postgres> = self.inner.pool.begin().await?;

        let _ = sqlx::query(self.inner.statements.delete_by_aggregate_id())
            .bind(aggregate_id)
            .execute(&mut *transaction)
            .await
            .map(|_| ())?;

        for projector in self.projectors().iter() {
            projector.delete(aggregate_id, &mut transaction).await?;
        }

        transaction.commit().await?;

        Ok(())
    }
}

/// Debug implementation for [`PgStore`]. It just shows the statements, that are the only thing
/// that might be useful to debug.
impl<T: AggregateManager> std::fmt::Debug for PgStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgStore")
            .field("statements", &self.inner.statements)
            .finish()
    }
}
