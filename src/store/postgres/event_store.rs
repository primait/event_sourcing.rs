use std::marker::PhantomData;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;
use futures::StreamExt;
use sqlx::pool::PoolConnection;
use sqlx::postgres::{PgAdvisoryLock, PgAdvisoryLockGuard, PgAdvisoryLockKey};
use sqlx::types::Json;
use sqlx::{Executor, PgConnection, Pool, Postgres, Transaction};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::bus::EventBus;
use crate::event::Event;
use crate::handler::{EventHandler, TransactionalEventHandler};
use crate::sql::event::DbEvent;
use crate::sql::statements::{Statements, StatementsHandler};
use crate::store::postgres::PgStoreError;
use crate::store::{EventStore, EventStoreLockGuard, StoreEvent, UnlockOnDrop};
use crate::types::SequenceNumber;
use crate::{Aggregate, AggregateState};

/// To support decoupling between the Aggregate::Event type and the schema of the DB table
/// in `PgStore` you can create a schema type that implements `Event` and `Schema`
/// where `E = Aggregate::Event`.
///
/// Note: Although `Schema::read` returns an `Option` for any given event and implementation.
///
/// The following must hold
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use esrs::store::postgres::Schema as SchemaTrait;
/// #
/// # #[derive(Clone, Eq, PartialEq, Debug)]
/// # struct Event {
/// #   a: u32,
/// # }
/// #
/// # #[derive(Serialize, Deserialize)]
/// # struct Schema {
/// #   a: u32,
/// # }
/// #
/// # #[cfg(feature = "upcasting")]
/// # impl esrs::event::Upcaster for Schema {}
/// #
/// # impl SchemaTrait<Event> for Schema {
/// #   fn write(Event { a }: Event) -> Self {
/// #     Self { a }
/// #   }
/// #
/// #   fn read(self) -> Option<Event> {
/// #     Some(Event { a: self.a })
/// #   }
/// # }
/// #
/// # let event = Event { a: 42 };
/// assert_eq!(Some(event.clone()), Schema::write(event).read());
/// ```
pub trait Schema<E>: Event {
    /// Converts the event into the schema type.
    fn write(event: E) -> Self;

    /// Converts the schema into the event type.
    ///
    /// This returns an option to enable skipping deprecated event which are persisted in the DB.
    ///
    /// Note: Although `Schema::read` returns an `Option` for any given event and implementation.
    ///
    /// The following must hold
    ///
    /// ```rust
    /// # use serde::{Serialize, Deserialize};
    /// # use esrs::store::postgres::Schema as SchemaTrait;
    /// #
    /// # #[derive(Clone, Eq, PartialEq, Debug)]
    /// # struct Event {
    /// #   a: u32,
    /// # }
    /// #
    /// # #[derive(Serialize, Deserialize)]
    /// # struct Schema {
    /// #   a: u32,
    /// # }
    /// #
    /// # #[cfg(feature = "upcasting")]
    /// # impl esrs::event::Upcaster for Schema {}
    /// #
    /// # impl SchemaTrait<Event> for Schema {
    /// #   fn write(Event { a }: Event) -> Self {
    /// #     Self { a }
    /// #   }
    /// #
    /// #   fn read(self) -> Option<Event> {
    /// #     Some(Event { a: self.a })
    /// #   }
    /// # }
    /// #
    /// # let event = Event { a: 42 };
    /// assert_eq!(Some(event.clone()), Schema::write(event).read());
    /// ```
    fn read(self) -> Option<E>;
}

impl<E> Schema<E> for E
where
    E: Event,
{
    fn write(event: E) -> Self {
        event
    }
    fn read(self) -> Option<E> {
        Some(self)
    }
}

/// Default Postgres implementation for the [`EventStore`]. Use this struct in order to have a
/// pre-made implementation of an [`EventStore`] persisting on Postgres.
///
/// The store is protected by an [`Arc`] that allows it to be cloneable still having the same memory
/// reference.
///
/// To decouple persistence from the event types, it is possible to optionally, specify the
/// Database event schema for this store as a type that implements `Event` and
/// `Schema<Aggregate::Event>`.
///
/// When events are persisted, they will first be converted to the `Schema` type using
/// `Schema::write` then serialized using the `Serialize` implementation on `Schema`.
///
/// When events are read from the store, they will first be deserialized into the `Schema` type and
/// then converted into an `Option<Aggregate::Event>` using `Schema::read`. In this way it is possible
/// to remove deprecate events in core part of your application by returning `None` from `Schema::read`.
pub struct PgStore<A, Schema = <A as Aggregate>::Event>
where
    A: Aggregate,
{
    pub(super) inner: Arc<InnerPgStore<A>>,
    pub(super) _schema: PhantomData<Schema>,
}

pub(super) struct InnerPgStore<A>
where
    A: Aggregate,
{
    pub(super) pool: Pool<Postgres>,
    pub(super) statements: Statements,
    pub(super) event_handlers: RwLock<Vec<Box<dyn EventHandler<A> + Send>>>,
    pub(super) transactional_event_handlers:
        Vec<Box<dyn TransactionalEventHandler<A, PgStoreError, PgConnection> + Send>>,
    pub(super) event_buses: Vec<Box<dyn EventBus<A> + Send>>,
}

impl<A, S> PgStore<A, S>
where
    A: Aggregate,
    A::Event: Send + Sync,
    S: Schema<A::Event> + Event + Send + Sync,
{
    /// Returns the name of the event store table
    pub fn table_name(&self) -> &str {
        self.inner.statements.table_name()
    }

    /// Safely add an event handler to [`PgStore`]. Since it appends an event handler to a [`RwLock`]
    /// this function needs to be `async`.
    ///
    /// This is mostly used while there's the need to have an event handler that try to apply a command
    /// on the same aggregate (implementing saga pattern with event sourcing).
    pub async fn add_event_handler(&self, event_handler: impl EventHandler<A> + Send + 'static) {
        let mut guard = self.inner.event_handlers.write().await;

        guard.push(Box::new(event_handler))
    }

    /// Save an event in the event store and return a new [`StoreEvent`] instance.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if the insert of the values into the database fails.
    pub(crate) async fn save_event(
        &self,
        aggregate_id: Uuid,
        event: A::Event,
        occurred_on: DateTime<Utc>,
        sequence_number: SequenceNumber,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<StoreEvent<A::Event>, PgStoreError> {
        let id: Uuid = Uuid::new_v4();

        #[cfg(feature = "upcasting")]
        let version: Option<i32> = S::current_version();
        #[cfg(not(feature = "upcasting"))]
        let version: Option<i32> = None;
        let schema = S::write(event);

        let _ = sqlx::query(self.inner.statements.insert())
            .bind(id)
            .bind(aggregate_id)
            .bind(Json(&schema))
            .bind(occurred_on)
            .bind(sequence_number)
            .bind(version)
            .execute(executor)
            .await?;

        Ok(StoreEvent {
            id,
            aggregate_id,
            payload: schema.read().expect(
                "For any type that implements Schema the following contract should be upheld:\
                assert_eq!(Some(event.clone()), Schema::write(event).read())",
            ),
            occurred_on,
            sequence_number,
            version,
        })
    }

    /// This function returns a stream representing the full event store table content. This should
    /// be mainly used to rebuild read models.
    pub fn stream_events<'s>(
        &'s self,
        executor: impl Executor<'s, Database = Postgres> + 's,
    ) -> BoxStream<Result<StoreEvent<A::Event>, PgStoreError>> {
        Box::pin({
            sqlx::query_as::<_, DbEvent>(self.inner.statements.select_all())
                .fetch(executor)
                .map(|res| Ok(res?.try_into_store_event::<_, S>()?))
                .map(Result::transpose)
                .filter_map(std::future::ready)
        })
    }
}

/// Concrete implementation of [`EventStoreLockGuard`] for the [`PgStore`].
///
/// It holds both the [`PgAdvisoryLock`] and its child [`PgAdvisoryLockGuard`].
/// When dropped, the [`PgAdvisoryLockGuard`] is dropped thus releasing the [`PgAdvisoryLock`].
#[ouroboros::self_referencing]
pub struct PgStoreLockGuard {
    lock: PgAdvisoryLock,
    #[borrows(lock)]
    #[covariant]
    guard: PgAdvisoryLockGuard<'this, PoolConnection<Postgres>>,
}

/// Marking [`PgStoreLockGuard`] as an [`UnlockOnDrop`] trait object.
impl UnlockOnDrop for PgStoreLockGuard {}

#[async_trait]
impl<A, S> EventStore for PgStore<A, S>
where
    A: Aggregate,
    A::State: Send,
    A::Event: Send + Sync,
    S: Schema<A::Event> + Event + Send + Sync,
{
    type Aggregate = A;
    type Error = PgStoreError;

    async fn lock(&self, aggregate_id: Uuid) -> Result<EventStoreLockGuard, Self::Error> {
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

    async fn by_aggregate_id(&self, aggregate_id: Uuid) -> Result<Vec<StoreEvent<A::Event>>, Self::Error> {
        Ok(sqlx::query_as::<_, DbEvent>(self.inner.statements.by_aggregate_id())
            .bind(aggregate_id)
            .fetch_all(&self.inner.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into_store_event::<_, S>()?))
            .filter_map(Result::transpose)
            .collect::<Result<Vec<StoreEvent<A::Event>>, Self::Error>>()?)
    }

    // Note: https://github.com/rust-lang/rust-clippy/issues/12281
    #[allow(clippy::blocks_in_conditions)]
    #[tracing::instrument(skip_all, fields(aggregate_id = % aggregate_state.id()), err)]
    async fn persist(
        &self,
        aggregate_state: &mut AggregateState<A::State>,
        events: Vec<A::Event>,
    ) -> Result<Vec<StoreEvent<A::Event>>, Self::Error> {
        let mut transaction: Transaction<Postgres> = self.inner.pool.begin().await?;
        let occurred_on: DateTime<Utc> = Utc::now();
        let mut store_events: Vec<StoreEvent<A::Event>> = vec![];

        let aggregate_id = *aggregate_state.id();

        for event in events.into_iter() {
            let store_event: StoreEvent<<A as Aggregate>::Event> = self
                .save_event(
                    aggregate_id,
                    event,
                    occurred_on,
                    aggregate_state.next_sequence_number(),
                    &mut *transaction,
                )
                .await?;

            store_events.push(store_event);
        }

        for store_event in &store_events {
            for transactional_event_handler in &self.inner.transactional_event_handlers {
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

        let event_handlers = self.inner.event_handlers.read().await;
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
            .inner
            .event_buses
            .iter()
            .map(|bus| async move {
                for store_event in store_events {
                    bus.publish(store_event).await;
                }
            })
            .collect();

        let _ = futures::future::join_all(futures).await;
    }

    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Self::Error> {
        let mut transaction: Transaction<Postgres> = self.inner.pool.begin().await?;

        let _ = sqlx::query(self.inner.statements.delete_by_aggregate_id())
            .bind(aggregate_id)
            .execute(&mut *transaction)
            .await
            .map(|_| ())?;

        for transactional_event_handler in self.inner.transactional_event_handlers.iter() {
            transactional_event_handler
                .delete(aggregate_id, &mut transaction)
                .await?;
        }

        transaction.commit().await?;

        let event_handlers = self.inner.event_handlers.read().await;
        // NOTE: should this be parallelized?
        for event_handler in event_handlers.iter() {
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

impl<A, S> Clone for PgStore<A, S>
where
    A: Aggregate,
{
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            _schema: PhantomData,
        }
    }
}
