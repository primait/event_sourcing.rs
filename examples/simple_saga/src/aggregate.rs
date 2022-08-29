use async_trait::async_trait;
use esrs::policy::SqlitePolicy;
use esrs::projector::SqliteProjector;
use sqlx::{Pool, Sqlite};
use uuid::Uuid;

use esrs::aggregate::{AggregateManager, AggregateState, Identifier};
use esrs::store::{EventStore, SqliteStore, StoreEvent};

use crate::structs::{LoggingCommand, LoggingError, LoggingEvent};

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
        let projectors: Vec<Box<dyn SqliteProjector<LoggingEvent, LoggingError> + Send + Sync>> = vec![]; // There are no projections here

        let policies: Vec<Box<dyn SqlitePolicy<LoggingEvent, LoggingError> + Send + Sync>> =
            vec![Box::new(LoggingPolicy {})];

        SqliteStore::new::<Self>(pool, projectors, policies).await
    }
}

impl Identifier for LoggingAggregate {
    fn name() -> &'static str {
        MESSAGES
    }
}

// A very simply policy, which tries to log an event, and creates another command after it finishes
// which indicates success or failure to log
struct LoggingPolicy;

#[async_trait]
impl SqlitePolicy<LoggingEvent, LoggingError> for LoggingPolicy {
    async fn handle_event(&self, event: &StoreEvent<LoggingEvent>, pool: &Pool<Sqlite>) -> Result<(), LoggingError> {
        let id = event.aggregate_id;
        let agg = LoggingAggregate::new(pool).await?;
        let state = agg
            .load(event.aggregate_id)
            .await
            .unwrap_or_else(|| AggregateState::new(event.aggregate_id)); // This should never happen
        match event.payload() {
            LoggingEvent::Received(msg) => {
                if msg.contains("fail_policy") {
                    agg.handle_command(state, LoggingCommand::Fail).await?;
                    return Err(LoggingError::Domain(msg.clone()));
                }
                println!("Logged via policy from {}: {}", id, msg);
                agg.handle_command(state, LoggingCommand::Succeed).await?;
            }
            _ => {}
        }
        Ok(())
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

    fn apply_event(_: &Uuid, state: Self::State, event: &StoreEvent<Self::Event>) -> Self::State {
        match event.payload() {
            LoggingEvent::Received(_) => {
                // Do nothing, since the logging policy is what carries out the side effect
                // NOTE that, in this case, the Self::State we return isn't valid any more,
                // since the policy runs another command on this aggregate, causing a state
                // update which isn't applied to the state returned here
            }
            LoggingEvent::Succeeded => {
                // No-op, we simply persist the event in the event log
                // This is where some success notification might be sent to another service,
                // or some other success handling would go, in a more complex system
            }
            LoggingEvent::Failed => {
                // No-op, we simply persist the event in the event log
                // This is where some cleanup action would go in a more complex system
            }
        }
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
            Self::Command::TryLog(msg) => self.persist(aggregate_state, vec![Self::Event::Received(msg)]).await,
            Self::Command::Succeed => self.persist(aggregate_state, vec![Self::Event::Succeeded]).await,
            Self::Command::Fail => self.persist(aggregate_state, vec![Self::Event::Failed]).await,
        }
    }
}
