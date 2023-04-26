use std::{sync::Arc, vec};

use sqlx::{postgres::PgQueryResult, PgConnection, Pool, Postgres, Transaction};

use super::{EventHandler, InnerPgStore, PgStore, TransactionalEventHandler};
use crate::{esrs::postgres::statement::Statements, Aggregate};

pub struct PgStoreBuilder<A>
where
    A: Aggregate,
{
    pool: Pool<Postgres>,
    statements: Statements,
    event_handlers: Vec<EventHandler<A>>,
    transactional_event_handlers: Vec<TransactionalEventHandler<A, PgConnection>>,
}

impl<A> PgStoreBuilder<A>
where
    A: Aggregate,
{
    pub fn new(pool: Pool<Postgres>) -> Self {
        PgStoreBuilder {
            pool,
            statements: Statements::new::<A>(),
            event_handlers: vec![],
            transactional_event_handlers: vec![],
        }
    }

    pub fn with_event_handlers(mut self, event_handlers: Vec<EventHandler<A>>) -> Self {
        self.event_handlers = event_handlers;
        self
    }

    pub fn add_event_handler(mut self, event_handler: EventHandler<A>) -> Self {
        self.event_handlers.push(event_handler);
        self
    }

    pub fn with_transactional_event_handlers(
        mut self,
        event_handlers: Vec<TransactionalEventHandler<A, PgConnection>>,
    ) -> Self {
        self.transactional_event_handlers = event_handlers;
        self
    }

    pub fn add_transactional_event_handler(
        mut self,
        event_handler: TransactionalEventHandler<A, PgConnection>,
    ) -> Self {
        self.transactional_event_handlers.push(event_handler);
        self
    }

    /// This function sets up the database in a transaction, creating the event store table (if not exists)
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
    pub async fn build(self) -> Result<PgStore<A>, sqlx::Error> {
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

        Ok(PgStore {
            inner: Arc::new(InnerPgStore {
                pool: self.pool,
                statements: self.statements,
                event_handlers: self.event_handlers,
                transactional_event_handlers: self.transactional_event_handlers,
            }),
        })
    }
}
