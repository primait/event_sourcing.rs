use async_trait::async_trait;
use sqlx::{Pool, Postgres};

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState};
use esrs::store::postgres::{PgStore, Policy, Projector};
use esrs::store::StoreEvent;

use crate::structs::{LoggingCommand, LoggingError, LoggingEvent};

pub struct LoggingAggregate {
    event_store: PgStore<Self>,
}

impl LoggingAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, LoggingError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    async fn new_store(pool: &Pool<Postgres>) -> Result<PgStore<Self>, LoggingError> {
        let projectors: Vec<Box<dyn Projector<Self> + Send + Sync>> = vec![]; // There are no projections here

        let policies: Vec<Box<dyn Policy<Self> + Send + Sync>> = vec![Box::new(LoggingPolicy {})];

        PgStore::new(pool, projectors, policies).setup().await
    }
}

// A very simply policy, which tries to log an event, and creates another command after it finishes
// which indicates success or failure to log
struct LoggingPolicy;

#[async_trait]
impl Policy<LoggingAggregate> for LoggingPolicy {
    async fn handle_event(&self, event: &StoreEvent<LoggingEvent>, pool: &Pool<Postgres>) -> Result<(), LoggingError> {
        let id = event.aggregate_id;
        let agg = LoggingAggregate::new(pool).await?;
        let state = agg
            .load(event.aggregate_id)
            .await
            .unwrap_or_else(|| AggregateState::new(event.aggregate_id)); // This should never happen

        if let LoggingEvent::Received(msg) = event.payload() {
            if msg.contains("fail_policy") {
                agg.handle(state, LoggingCommand::Fail).await?;
                return Err(LoggingError::Domain(msg.clone()));
            }
            println!("Logged via policy from {}: {}", id, msg);
            agg.handle(state, LoggingCommand::Succeed).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl Aggregate for LoggingAggregate {
    type State = u64;
    type Command = LoggingCommand;
    type Event = LoggingEvent;
    type Error = LoggingError;

    fn name() -> &'static str {
        "message"
    }

    fn handle_command(
        _state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::TryLog(msg) => Ok(vec![Self::Event::Received(msg)]),
            Self::Command::Succeed => Ok(vec![Self::Event::Succeeded]),
            Self::Command::Fail => Ok(vec![Self::Event::Failed]),
        }
    }

    fn apply_event(state: Self::State, event: Self::Event) -> Self::State {
        match event {
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
}

impl AggregateManager for LoggingAggregate {
    type EventStore = PgStore<Self>;

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}
