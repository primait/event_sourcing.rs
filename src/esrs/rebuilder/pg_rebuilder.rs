use async_trait::async_trait;
use futures::StreamExt;
use sqlx::{PgConnection, Pool, Postgres, Transaction};
use uuid::Uuid;

use crate::esrs::event_bus::EventBus;
use crate::esrs::rebuilder::Rebuilder;
use crate::postgres::{PgStore, PgStoreBuilder};
use crate::{Aggregate, EventStore, ReplayableEventHandler, StoreEvent, TransactionalEventHandler};

pub struct PgRebuilder<A>
where
    A: Aggregate,
{
    event_handlers: Vec<Box<dyn ReplayableEventHandler<A> + Send>>,
    transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, PgConnection> + Send>>,
    event_buses: Vec<Box<dyn EventBus<A> + Send>>,
}

impl<A> PgRebuilder<A>
where
    A: Aggregate,
{
    pub fn new() -> Self {
        Self {
            event_handlers: vec![],
            transactional_event_handlers: vec![],
            event_buses: vec![],
        }
    }

    pub fn with_event_handlers(self, event_handlers: Vec<Box<dyn ReplayableEventHandler<A> + Send>>) -> Self {
        Self { event_handlers, ..self }
    }

    pub fn with_transactional_event_handlers(
        self,
        transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, PgConnection> + Send>>,
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

#[async_trait]
impl<A> Rebuilder<A, Pool<Postgres>> for PgRebuilder<A>
where
    A: Aggregate,
    A::Event: serde::Serialize + serde::de::DeserializeOwned + Send,
    A::Error: From<sqlx::Error> + From<serde_json::Error> + std::error::Error + Send,
{
    async fn by_aggregate_id(&self, pool: Pool<Postgres>) -> Result<(), A::Error> {
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

    async fn all_at_once(&self, pool: Pool<Postgres>) -> Result<(), A::Error> {
        let store: PgStore<A> = PgStoreBuilder::new(pool.clone())
            .without_running_migrations()
            .try_build()
            .await?;

        let mut transaction: Transaction<Postgres> = pool.begin().await.unwrap();

        let events: Vec<StoreEvent<A::Event>> = store
            .stream_events(&mut transaction)
            .collect::<Vec<Result<StoreEvent<A::Event>, A::Error>>>()
            .await
            .into_iter()
            .collect::<Result<Vec<StoreEvent<A::Event>>, A::Error>>()?;

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