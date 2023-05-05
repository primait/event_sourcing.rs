use std::{sync::Arc, vec};

use sqlx::{PgConnection, Pool, Postgres};

use crate::esrs::sql::migrations::{Migrations, MigrationsHandler};
use crate::esrs::sql::statements::Statements;
use crate::Aggregate;

use super::{EventBus, EventHandler, InnerPgStore, PgStore, TransactionalEventHandler};

pub struct PgStoreBuilder<A>
where
    A: Aggregate,
{
    pool: Pool<Postgres>,
    statements: Statements,
    event_handlers: Vec<EventHandler<A>>,
    transactional_event_handlers: Vec<TransactionalEventHandler<A, PgConnection>>,
    event_buses: Vec<EventBus<A>>,
    run_migrations: bool,
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
            event_buses: vec![],
            run_migrations: true,
        }
    }

    /// Set event handlers list
    pub fn with_event_handlers(mut self, event_handlers: Vec<EventHandler<A>>) -> Self {
        self.event_handlers = event_handlers;
        self
    }

    /// Add a single event handler
    pub fn add_event_handler(mut self, event_handler: EventHandler<A>) -> Self {
        self.event_handlers.push(event_handler);
        self
    }

    /// Set transactional event handlers list
    pub fn with_transactional_event_handlers(
        mut self,
        transactional_event_handlers: Vec<TransactionalEventHandler<A, PgConnection>>,
    ) -> Self {
        self.transactional_event_handlers = transactional_event_handlers;
        self
    }

    /// Add a single transactional event handler
    pub fn add_transactional_event_handler(
        mut self,
        transaction_event_handler: TransactionalEventHandler<A, PgConnection>,
    ) -> Self {
        self.transactional_event_handlers.push(transaction_event_handler);
        self
    }

    /// Set event buses list
    pub fn with_event_buses(mut self, event_buses: Vec<EventBus<A>>) -> Self {
        self.event_buses = event_buses;
        self
    }

    /// Add a single event bus
    pub fn add_event_bus(mut self, event_bus: EventBus<A>) -> Self {
        self.event_buses.push(event_bus);
        self
    }

    /// Calling this function the caller avoid running migrations. It is recommend to run migrations
    /// at least once per store per startup.
    pub fn without_running_migrations(mut self) -> Self {
        self.run_migrations = false;
        self
    }

    /// This function sets up the database in a transaction and returns an instance of PgStore.
    ///
    /// It will create the event store table (if it doesn't exist) and two indexes (if they don't exist).
    /// The first one is over the `aggregate_id` field to speed up `by_aggregate_id` query.
    /// The second one is a unique constraint over the tuple `(aggregate_id, sequence_number)` to avoid race conditions.
    ///
    /// This function should be used only once at your application startup.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if there's an error connecting with database or creating tables/indexes.
    pub async fn try_build(self) -> Result<PgStore<A>, sqlx::Error> {
        if self.run_migrations {
            Migrations::run::<A>(&self.pool).await?;
        }

        Ok(PgStore {
            inner: Arc::new(InnerPgStore {
                pool: self.pool,
                statements: self.statements,
                event_handlers: self.event_handlers,
                transactional_event_handlers: self.transactional_event_handlers,
                event_buses: self.event_buses,
            }),
        })
    }
}
