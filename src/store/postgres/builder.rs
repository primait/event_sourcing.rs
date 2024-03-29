use std::sync::Arc;

use sqlx::{PgConnection, Pool, Postgres};
use tokio::sync::RwLock;

use crate::bus::EventBus;
use crate::handler::{EventHandler, TransactionalEventHandler};
use crate::sql::migrations::{Migrations, MigrationsHandler};
use crate::sql::statements::{Statements, StatementsHandler};
use crate::store::postgres::{InnerPgStore, PgStoreError};
use crate::Aggregate;

use super::PgStore;

/// Struct used to build a brand new [`PgStore`].
pub struct PgStoreBuilder<A>
where
    A: Aggregate,
{
    pool: Pool<Postgres>,
    statements: Statements,
    event_handlers: Vec<Box<dyn EventHandler<A> + Send>>,
    transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, PgStoreError, PgConnection> + Send>>,
    event_buses: Vec<Box<dyn EventBus<A> + Send>>,
    run_migrations: bool,
}

impl<A> PgStoreBuilder<A>
where
    A: Aggregate,
{
    /// Creates a new instance of a [`PgStoreBuilder`].
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
    pub fn with_event_handlers(mut self, event_handlers: Vec<Box<dyn EventHandler<A> + Send>>) -> Self {
        self.event_handlers = event_handlers;
        self
    }

    /// Add a single event handler
    pub fn add_event_handler(mut self, event_handler: impl EventHandler<A> + Send + 'static) -> Self {
        self.event_handlers.push(Box::new(event_handler));
        self
    }

    /// Set transactional event handlers list
    pub fn with_transactional_event_handlers(
        mut self,
        transactional_event_handlers: Vec<Box<dyn TransactionalEventHandler<A, PgStoreError, PgConnection> + Send>>,
    ) -> Self {
        self.transactional_event_handlers = transactional_event_handlers;
        self
    }

    /// Add a single transactional event handler
    pub fn add_transactional_event_handler(
        mut self,
        transaction_event_handler: impl TransactionalEventHandler<A, PgStoreError, PgConnection> + Send + 'static,
    ) -> Self {
        self.transactional_event_handlers
            .push(Box::new(transaction_event_handler));
        self
    }

    /// Set event buses list
    pub fn with_event_buses(mut self, event_buses: Vec<Box<dyn EventBus<A> + Send>>) -> Self {
        self.event_buses = event_buses;
        self
    }

    /// Add a single event bus
    pub fn add_event_bus(mut self, event_bus: impl EventBus<A> + Send + 'static) -> Self {
        self.event_buses.push(Box::new(event_bus));
        self
    }

    /// Calling this function the caller avoid running migrations. It is recommend to run migrations
    /// at least once per store per startup.
    pub fn without_running_migrations(mut self) -> Self {
        self.run_migrations = false;
        self
    }

    /// This function runs all the needed [`Migrations`], atomically setting up the database if
    /// `run_migrations` isn't explicitly set to false. [`Migrations`] should be run only at application
    /// startup due to avoid performance issues.
    ///
    /// Eventually returns an instance of PgStore.
    ///
    /// # Errors
    ///
    /// Will return an `Err` if there's an error running [`Migrations`].
    pub async fn try_build(self) -> Result<PgStore<A>, sqlx::Error> {
        if self.run_migrations {
            Migrations::run::<A>(&self.pool).await?;
        }

        Ok(PgStore {
            inner: Arc::new(InnerPgStore {
                pool: self.pool,
                statements: self.statements,
                event_handlers: RwLock::new(self.event_handlers),
                transactional_event_handlers: self.transactional_event_handlers,
                event_buses: self.event_buses,
            }),
        })
    }
}
