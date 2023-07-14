use async_trait::async_trait;
use futures::StreamExt;
use sqlx::{PgConnection, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::bus::EventBus;
use crate::handler::{ReplayableEventHandler, TransactionalEventHandler};
use crate::rebuilder::Rebuilder;
use crate::store::postgres::{PgStore, PgStoreBuilder, PgStoreError};
use crate::store::{EventStore, StoreEvent};
use crate::Aggregate;

pub struct PgRebuilder<A>
where
    A: Aggregate,
{
    event_handlers: Vec<Box<dyn ReplayableEventHandler<A> + Send>>,
    transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, PgStoreError, PgConnection> + Send>>,
    event_buses: Vec<Box<dyn EventBus<A> + Send>>,
}

impl<A> PgRebuilder<A>
where
    A: Aggregate,
{
    pub fn new() -> Self {
        Default::default()
    }

    pub fn with_event_handlers(self, event_handlers: Vec<Box<dyn ReplayableEventHandler<A> + Send>>) -> Self {
        Self { event_handlers, ..self }
    }

    pub fn with_transactional_event_handlers(
        self,
        transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, PgStoreError, PgConnection> + Send>>,
    ) -> Self {
        Self {
            transactional_event_handlers,
            ..self
        }
    }

    pub fn with_event_buses(self, event_buses: Vec<Box<dyn EventBus<A> + Send>>) -> Self {
        Self { event_buses, ..self }
    }
}

impl<A> Default for PgRebuilder<A>
where
    A: Aggregate,
{
    fn default() -> Self {
        Self {
            event_handlers: vec![],
            transactional_event_handlers: vec![],
            event_buses: vec![],
        }
    }
}

#[async_trait]
impl<A> Rebuilder<A> for PgRebuilder<A>
where
    A: Aggregate,
    A::Event: serde::Serialize + serde::de::DeserializeOwned + Send + Sync,
    A::State: Send,
{
    type Executor = Pool<Postgres>;
    type Error = PgStoreError;

    /// To optimize performance, the code can be modified to open a single transaction for all the
    /// aggregate IDs fetched by a pre-made query. Within this transaction, the list of events for
    /// each aggregate ID is extracted. Then, for every [`TransactionalEventHandler`] and [`EventHandler`],
    /// the corresponding aggregate is deleted, and the list of events is processed by the mentioned
    /// handlers.
    /// Finally the events are passed to every configured [`EventBus`].
    async fn by_aggregate_id(&self, pool: Pool<Postgres>) -> Result<(), Self::Error> {
        let store: PgStore<A> = PgStoreBuilder::new(pool.clone())
            .without_running_migrations()
            .try_build()
            .await?;

        let aggregate_ids: Vec<Uuid> = get_all_aggregate_ids(&pool, store.table_name()).await?;

        for id in aggregate_ids {
            let mut transaction: Transaction<Postgres> = pool.begin().await.unwrap();

            let events = store.by_aggregate_id(id).await.unwrap();

            for handler in self.transactional_event_handlers.iter() {
                handler.delete(id, &mut transaction).await?;

                for event in &events {
                    handler.handle(event, &mut transaction).await?;
                }
            }

            transaction.commit().await.unwrap();

            for handler in self.event_handlers.iter() {
                handler.delete(id).await;

                for event in &events {
                    handler.handle(event).await;
                }
            }

            for bus in self.event_buses.iter() {
                for event in &events {
                    bus.publish(event).await;
                }
            }
        }

        Ok(())
    }

    /// To process all events in the database, a single transaction is opened, and within this
    /// transaction, all aggregates are deleted and for each [`TransactionalEventHandler`], the
    /// events are handled. After the transaction ends, for each [`EventHandler`] and [`EventBus`],
    /// the events are handled.
    async fn all_at_once(&self, pool: Pool<Postgres>) -> Result<(), Self::Error> {
        let store: PgStore<A> = PgStoreBuilder::new(pool.clone())
            .without_running_migrations()
            .try_build()
            .await?;

        let mut transaction: Transaction<Postgres> = pool.begin().await.unwrap();

        let events: Vec<StoreEvent<A::Event>> = store
            .stream_events(&mut *transaction)
            .collect::<Vec<Result<StoreEvent<A::Event>, Self::Error>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<StoreEvent<A::Event>>, Self::Error>>()?;

        for event in &events {
            for handler in self.transactional_event_handlers.iter() {
                handler.delete(event.aggregate_id, &mut transaction).await?;
                handler.handle(event, &mut transaction).await?;
            }
        }

        transaction.commit().await?;

        for event in &events {
            for handler in self.event_handlers.iter() {
                handler.delete(event.aggregate_id).await;
                handler.handle(event).await;
            }

            for bus in self.event_buses.iter() {
                for event in &events {
                    bus.publish(event).await;
                }
            }
        }

        Ok(())
    }
}

async fn get_all_aggregate_ids(pool: &Pool<Postgres>, store_table_name: &str) -> Result<Vec<Uuid>, sqlx::Error> {
    let query: String = format!("SELECT DISTINCT(aggregate_id) FROM {}", store_table_name);
    let result: Vec<(Uuid,)> = sqlx::query_as::<_, (Uuid,)>(query.as_str()).fetch_all(pool).await?;
    Ok(result.iter().map(|v| v.0).collect())
}
