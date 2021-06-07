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
use sqlx::postgres::PgDone;
use sqlx::Postgres;
use uuid::Uuid;

use policy::PgPolicy;
use projector::PgProjector;

use crate::esrs::aggregate::Identifier;
use crate::esrs::event::Event;
use crate::esrs::pool::{Pool, Transaction};
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
    Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
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
}

impl<
        'a,
        Evt: 'a + Serialize + DeserializeOwned + Clone + Send + Sync,
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
        // Check if table and indexes exist and eventually create them
        util::run_preconditions(pool, aggregate_name).await?;

        Ok(Self {
            pool: pool.clone(),
            projectors,
            policies,
            queries: Queries::new(aggregate_name),
            evt: PhantomData::default(),
            err: PhantomData::default(),
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

    /// Begin a new transaction. Commit returned transaction or Drop will automatically rollback it
    pub async fn begin(&self) -> Result<Transaction<'_, Postgres>, sqlx::Error> {
        self.pool.begin().await
    }

    pub async fn rebuild_events(&self) -> Result<(), Err> {
        let mut events: BoxStream<Result<Event, sqlx::Error>> =
            sqlx::query_as::<_, Event>(self.queries.select_all()).fetch(&*self.pool);

        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        while let Some(event) = events.try_next().await? {
            let evt: StoreEvent<Evt> = event.try_into()?;
            self.project_event(&evt, &mut transaction).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl<
        Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjector<Evt, Err> + Send + Sync + ?Sized,
        Policy: PgPolicy<Evt, Err> + Send + Sync + ?Sized,
    > EventStore<Evt, Err> for InnerPgStore<Evt, Err, Projector, Policy>
{
    async fn by_aggregate_id(&self, id: Uuid) -> Result<Vec<StoreEvent<Evt>>, Err> {
        Ok(sqlx::query_as::<_, Event>(self.queries.select())
            .bind(id)
            .fetch_all(&*self.pool)
            .await?
            .into_iter()
            .map(|event| Ok(event.try_into()?))
            .collect::<Result<Vec<StoreEvent<Evt>>, Err>>()?)
    }

    async fn persist(
        &self,
        aggregate_id: Uuid,
        event: Evt,
        sequence_number: SequenceNumber,
    ) -> Result<StoreEvent<Evt>, Err> {
        let mut transaction: Transaction<Postgres> = self.pool.begin().await?;

        let event_id: Uuid = Uuid::new_v4();
        let occurred_on: DateTime<Utc> = Utc::now();
        let store_event_result: Result<PgDone, Err> = sqlx::query(self.queries.insert())
            .bind(event_id)
            .bind(aggregate_id)
            .bind(serde_json::to_value(event.clone()).unwrap())
            .bind(Utc::now())
            .bind(sequence_number)
            .execute(&mut *transaction)
            .await
            .map_err(|error| error.into());

        let rebuild_result: Result<StoreEvent<Evt>, Err> = match store_event_result {
            Ok(_) => {
                let store_event: StoreEvent<Evt> = StoreEvent {
                    id: event_id,
                    aggregate_id,
                    payload: event.clone(),
                    occurred_on,
                    sequence_number,
                };

                self.project_event(&store_event, &mut transaction)
                    .await
                    .map(|()| store_event)
            }
            Err(error) => Err(error),
        };

        match rebuild_result {
            Ok(event) => {
                transaction.commit().await?;

                for policy in &self.policies {
                    policy.handle_event(&event, &self.pool).await?
                }

                Ok(event)
            }
            Err(err) => {
                transaction.rollback().await?;
                Err(err)
            }
        }
    }

    async fn close(&self) {
        self.pool.close().await
    }
}

impl<
        'c,
        Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjector<Evt, Err> + Send + Sync + ?Sized,
        Policy: PgPolicy<Evt, Err> + Send + Sync + ?Sized,
    > ProjectorStore<Evt, Transaction<'c, Postgres>, Err> for InnerPgStore<Evt, Err, Projector, Policy>
{
    fn project_event<'a>(
        &'a self,
        store_event: &'a StoreEvent<Evt>,
        executor: &'a mut Transaction<'c, Postgres>,
    ) -> Pin<Box<dyn Future<Output = Result<(), Err>> + Send + 'a>>
    where
        Self: Sync + 'a,
    {
        async fn run<
            Ev: Serialize + DeserializeOwned + Clone + Send + Sync,
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

        Box::pin(run::<Evt, Err, Projector, Policy>(self, store_event, executor))
    }
}

#[async_trait]
impl<
        Evt: Serialize + DeserializeOwned + Clone + Send + Sync,
        Err: From<sqlx::Error> + From<serde_json::Error> + Send + Sync,
        Projector: PgProjectorEraser<Evt, Err> + Send + Sync + ?Sized,
        Policy: PgPolicy<Evt, Err> + Send + Sync + ?Sized,
    > EraserStore<Evt, Err> for InnerPgStore<Evt, Err, Projector, Policy>
{
    async fn delete(&self, aggregate_id: Uuid) -> Result<(), Err> {
        let mut transaction: Transaction<Postgres> = self.begin().await?;
        let _ = sqlx::query(self.queries.delete())
            .bind(aggregate_id)
            .execute(&mut *transaction)
            .await
            .map(|_| ());

        for projector in &self.projectors {
            projector.delete(aggregate_id, &mut transaction).await?
        }

        Ok(transaction.commit().await?)
    }
}
