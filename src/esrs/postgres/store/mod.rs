use std::convert::TryInto;
use std::future::Future;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::StreamExt;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgAdvisoryLock, PgAdvisoryLockGuard, PgAdvisoryLockKey};
use sqlx::types::Json;
use sqlx::{Executor, PgConnection, Pool, Postgres, Transaction};
use uuid::Uuid;

pub use builder::PgStoreBuilder;

use crate::esrs::event_handler;
use crate::esrs::sql::statements::Statements;
use crate::esrs::store::{EventStoreLockGuard, UnlockOnDrop};
use crate::types::SequenceNumber;
use crate::{Aggregate, AggregateState, EventStore, StoreEvent};

use super::event;

mod builder;

pub type EventHandler<A> = Box<dyn event_handler::EventHandler<A> + Send + Sync>;
pub type TransactionalEventHandler<A, E> = Box<dyn event_handler::TransactionalEventHandler<A, E> + Send + Sync>;
pub type EventBus<A> = Box<dyn crate::esrs::event_bus::EventBus<A> + Send + Sync>;

/// Default Postgres implementation for the [`EventStore`]. Use this struct in order to have a
/// pre-made implementation of an [`EventStore`] persisting on Postgres.
///
/// The store is protected by an [`Arc`] that allows it to be cloneable still having the same memory
/// reference.
#[derive(Clone)]
pub struct PgStore<A>
where
    A: Aggregate,
{
    inner: Arc<InnerPgStore<A>>,
}

struct InnerPgStore<A>
where
    A: Aggregate,
{
    pool: Pool<Postgres>,
    statements: Statements,
    event_handlers: Vec<EventHandler<A>>,
    transactional_event_handlers: Vec<TransactionalEventHandler<A, PgConnection>>,
    event_buses: Vec<EventBus<A>>,
}

impl<A> PgStore<A>
where
    A: Aggregate,
    A::Event: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
    A::Error: From<sqlx::Error> + From<serde_json::Error> + std::error::Error,
{
    /// Save an event in the event store and return a new `StoreEvent` instance.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the insert of the values into the database fails.
    pub async fn save_event(
        &self,
        aggregate_id: Uuid,
        event: A::Event,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<StoreEvent<A::Event>, A::Error> {
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
    ) -> BoxStream<Result<StoreEvent<A::Event>, A::Error>> {
        Box::pin({
            sqlx::query_as::<_, event::Event>(self.inner.statements.select_all())
                .fetch(executor)
                .map(|res| Ok(res?.try_into()?))
        })
    }

    /// This function returns the list of all transactional event handlers added to this store.
    /// This function should mostly used while creating a custom persistence flow using [`PgStore::persist`].
    pub fn transactional_event_handlers(&self) -> &[TransactionalEventHandler<A, PgConnection>] {
        &self.inner.transactional_event_handlers
    }

    /// This function returns the list of all event handlers added to this store. This function should
    /// mostly used while creating a custom persistence flow using [`PgStore::persist`].
    pub fn event_handlers(&self) -> &[EventHandler<A>] {
        &self.inner.event_handlers
    }

    /// This function returns the list of all event handlers added to this store. This function should
    /// mostly used while creating a custom persistence flow using [`PgStore::persist`].
    pub fn event_buses(&self) -> &[EventBus<A>] {
        &self.inner.event_buses
    }

    /// This function could be used in order to customize the way the store persist the events.
    ///
    /// An example of how to use this function is in `examples/customize_persistence_flow` example folder.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the given `fun` returns an `Err`. In the `EventStore` implementation
    /// for `PgStore` this function return an `Err` if the event insertion or its projection fails.
    pub async fn persist<'a, F, T>(&'a self, fun: F) -> Result<Vec<StoreEvent<A::Event>>, A::Error>
    where
        F: Send + FnOnce(&'a Pool<Postgres>) -> T,
        T: Future<Output = Result<Vec<StoreEvent<A::Event>>, A::Error>> + Send,
    {
        fun(&self.inner.pool).await
    }
}

/// Concrete implementation of EventStoreLockGuard for the PgStore.
///
/// It holds both the PgAdvisoryLock and its child PgAdvisoryLockGuard.
/// When dropped, the PgAdvisoryLockGuard is dropped thus releasing the PgAdvisoryLock.
#[ouroboros::self_referencing]
pub struct PgStoreLockGuard {
    lock: PgAdvisoryLock,
    #[borrows(lock)]
    #[covariant]
    guard: PgAdvisoryLockGuard<'this, PoolConnection<Postgres>>,
}

/// Marking PgStoreLockGuard as an UnlockOnDrop trait object.
impl UnlockOnDrop for PgStoreLockGuard {}

#[async_trait]
impl<A> EventStore<A> for PgStore<A>
where
    A: Aggregate,
    A::Event: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
    A::Error: From<sqlx::Error> + From<serde_json::Error> + std::error::Error,
{
    async fn lock(&self, aggregate_id: Uuid) -> Result<EventStoreLockGuard, A::Error> {
        let (key, _) = aggregate_id.as_u64_pair();
        let connection = self.inner.pool.acquire().await?;
        let lock_guard = PgStoreLockGuardAsyncSendTryBuilder {
            lock: PgAdvisoryLock::with_key(PgAdvisoryLockKey::BigInt(key as i64)),
            guard_builder: |lock: &PgAdvisoryLock| Box::pin(async move { lock.acquire(connection).await }),
        }
        .try_build()
        .await?;
        Ok(EventStoreLockGuard::new(lock_guard))
    }

    async fn by_aggregate_id(&self, aggregate_id: Uuid) -> Result<Vec<StoreEvent<A::Event>>, A::Error> {
        Ok(
            sqlx::query_as::<_, event::Event>(self.inner.statements.by_aggregate_id())
                .bind(aggregate_id)
                .fetch_all(&self.inner.pool)
                .await?
                .into_iter()
                .map(|event| Ok(event.try_into()?))
                .collect::<Result<Vec<StoreEvent<A::Event>>, A::Error>>()?,
        )
    }

    #[tracing::instrument(skip_all, fields(aggregate_id = %aggregate_state.id()), err)]
    async fn persist(
        &self,
        aggregate_state: &mut AggregateState<A::State>,
        events: Vec<A::Event>,
    ) -> Result<Vec<StoreEvent<A::Event>>, A::Error> {
        let mut transaction: Transaction<Postgres> = self.inner.pool.begin().await?;
        let occurred_on: DateTime<Utc> = Utc::now();
        let mut store_events: Vec<StoreEvent<A::Event>> = vec![];

        let starting_sequence_number = aggregate_state.next_sequence_number();
        let aggregate_id = *aggregate_state.id();

        for (index, event) in (0..).zip(events.into_iter()) {
            let store_event: StoreEvent<<A as Aggregate>::Event> = self
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

        // Acquiring the list of transactional event handlers early, as it is an expensive operation.
        let transactional_event_handlers = self.transactional_event_handlers();
        for store_event in &store_events {
            for transactional_event_handler in transactional_event_handlers.iter() {
                let span = tracing::trace_span!(
                    "esrs.transactional_event_handler",
                    event_id = %store_event.id,
                    aggregate_id = %store_event.aggregate_id,
                    transactional_event_handler = transactional_event_handler.name()
                );
                let _e = span.enter();

                if let Err(error) = transactional_event_handler.handle(store_event, &mut transaction).await {
                    tracing::error!({
                        event_id = %store_event.id,
                        aggregate_id = %store_event.aggregate_id,
                        transactional_event_handler = transactional_event_handler.name(),
                        error = ?error,
                    }, "transactional event handler failed to handle event");

                    return Err(error);
                }
            }
        }

        transaction.commit().await?;

        // We need to drop the lock on the aggregate state here as:
        // 1. the events have already been persisted, hence the DB has the latest aggregate;
        // 2. the event handlers below might need to access this aggregate atomically (causing a deadlock!).
        drop(aggregate_state.take_lock());

        // Acquiring the list of event handlers early, as it is an expensive operation.
        let event_handlers = self.event_handlers();
        for store_event in &store_events {
            // NOTE: should this be parallelized?
            for event_handler in event_handlers.iter() {
                let span = tracing::debug_span!(
                    "esrs.event_handler",
                    event_id = %store_event.id,
                    aggregate_id = %store_event.aggregate_id,
                    event_handler = event_handler.name()
                );
                let _e = span.enter();

                event_handler.handle(store_event).await;
            }
        }

        // Publishing to subscribed event buses
        self.publish(&store_events).await;

        Ok(store_events)
    }

    async fn publish(&self, store_events: &[StoreEvent<A::Event>]) {
        let futures: Vec<_> = self
            .event_buses()
            .iter()
            .map(|bus| async move {
                for store_event in store_events {
                    bus.publish(store_event).await;
                }
            })
            .collect();

        let _ = futures::future::join_all(futures).await;
    }

    async fn delete(&self, aggregate_id: Uuid) -> Result<(), A::Error> {
        let mut transaction: Transaction<Postgres> = self.inner.pool.begin().await?;

        let _ = sqlx::query(self.inner.statements.delete_by_aggregate_id())
            .bind(aggregate_id)
            .execute(&mut *transaction)
            .await
            .map(|_| ())?;

        for transactional_event_handler in self.transactional_event_handlers().iter() {
            transactional_event_handler
                .delete(aggregate_id, &mut transaction)
                .await?;
        }

        transaction.commit().await?;

        // NOTE: should this be parallelized?
        for event_handler in self.event_handlers().iter() {
            event_handler.delete(aggregate_id).await;
        }

        Ok(())
    }
}

/// Debug implementation for [`PgStore`]. It just shows the statements, that are the only thing
/// that might be useful to debug.
impl<T: Aggregate> std::fmt::Debug for PgStore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PgStore")
            .field("statements", &self.inner.statements)
            .finish()
    }
}
