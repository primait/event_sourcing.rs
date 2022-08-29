use async_trait::async_trait;
use esrs::policy::SqlitePolicy;
use esrs::projector::SqliteProjector;
use sqlx::pool::PoolConnection;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use esrs::aggregate::{AggregateManager, AggregateState, Identifier};
use esrs::store::{EventStore, SqliteStore, StoreEvent};

use crate::structs::{LoggingCommand, LoggingError, LoggingEvent};

// Here's a simple projection that just carries out a side effect, and can fail,
// causing the event not to be written to the event store. In this case the
// failure is due to a simple log message filter rule, but you can imagine a
// side effect which interacts with some 3rd party service in a failable way instead
pub struct LoggingProjector;

#[async_trait]
impl SqliteProjector<LoggingEvent, LoggingError> for LoggingProjector {
    async fn project(
        &self,
        event: &StoreEvent<LoggingEvent>,
        _: &mut PoolConnection<Sqlite>,
    ) -> Result<(), LoggingError> {
        let id = event.aggregate_id;
        match event.payload() {
            LoggingEvent::Logged(msg) => {
                if msg.contains("fail_projection") {
                    return Err(LoggingError::Domain(msg.clone()));
                }
                println!("Logged via projector from {}: {}", id, msg);
            }
        }
        Ok(())
    }
}

// Here's a simple side-effect policy which carries out some side effect.
// This is also failable - in a similar way to the projection above, however,
// if this fails, the event is still persisted, and then the aggregate state
// is lost. The largest difference between a failure in a projection and a
// failure in a policy, from an aggregate perspective, is that a failed projection
// stops the event from being persisted to the event store, whereas a failure in
// a policy does not.
pub struct LoggingPolicy;

#[async_trait]
impl SqlitePolicy<LoggingEvent, LoggingError> for LoggingPolicy {
    async fn handle_event(&self, event: &StoreEvent<LoggingEvent>, _: &Pool<Sqlite>) -> Result<(), LoggingError> {
        let id = event.aggregate_id;
        match event.payload() {
            LoggingEvent::Logged(msg) => {
                if msg.contains("fail_policy") {
                    return Err(LoggingError::Domain(msg.clone()));
                }
                println!("Logged via policy from {}: {}", id, msg);
            }
        }
        Ok(())
    }
}

const MESSAGES: &str = "message";

// A store of events
pub type LogStore = SqliteStore<LoggingEvent, LoggingError>;

pub struct LoggingAggregate {
    event_store: LogStore,
}

impl LoggingAggregate {
    pub async fn new(pool: &Pool<Sqlite>) -> Result<Self, LoggingError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    pub async fn new_store(pool: &Pool<Sqlite>) -> Result<LogStore, LoggingError> {
        let projectors: Vec<Box<dyn SqliteProjector<LoggingEvent, LoggingError> + Send + Sync>> =
            vec![Box::new(LoggingProjector)];

        let policies: Vec<Box<dyn SqlitePolicy<LoggingEvent, LoggingError> + Send + Sync>> =
            vec![Box::new(LoggingPolicy)];

        SqliteStore::new::<Self>(pool, projectors, policies).await
    }
}

impl Identifier for LoggingAggregate {
    fn name() -> &'static str {
        MESSAGES
    }
}

#[async_trait]
impl AggregateManager for LoggingAggregate {
    type State = u64;
    type Command = LoggingCommand;
    type Event = LoggingEvent;
    type Error = LoggingError;

    fn event_store(&self) -> &(dyn EventStore<Self::Event, Self::Error> + Send + Sync) {
        &self.event_store
    }

    fn apply_event(_: &Uuid, state: Self::State, _: &StoreEvent<Self::Event>) -> Self::State {
        // This aggregate state just counts the number of applied - equivalent to the number in the event store
        state + 1
    }

    fn validate_command(_: &AggregateState<Self::State>, _: &Self::Command) -> Result<(), Self::Error> {
        Ok(()) // No validation done on commands received in this aggregate
    }

    async fn do_handle_command(
        &self,
        aggregate_state: AggregateState<Self::State>,
        cmd: Self::Command,
    ) -> Result<AggregateState<Self::State>, Self::Error> {
        match cmd {
            Self::Command::Log(msg) => self.persist(aggregate_state, vec![Self::Event::Logged(msg)]).await,
        }
    }
}
