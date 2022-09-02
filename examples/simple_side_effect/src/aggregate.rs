use async_trait::async_trait;
use sqlx::{Pool, Postgres, Transaction};

use esrs::aggregate::{Aggregate, AggregateManager, AggregateState};
use esrs::policy::Policy;
use esrs::projector::Projector;
use esrs::store::{PgStore, StoreEvent};

use crate::structs::{LoggingCommand, LoggingError, LoggingEvent};

// Here's a simple projection that just carries out a side effect, and can fail,
// causing the event not to be written to the event store. In this case the
// failure is due to a simple log message filter rule, but you can imagine a
// side effect which interacts with some 3rd party service in a failable way instead
pub struct LoggingProjector;

#[async_trait]
impl Projector<Postgres, LoggingEvent, LoggingError> for LoggingProjector {
    async fn project(
        &self,
        event: &StoreEvent<LoggingEvent>,
        _: &mut Transaction<'_, Postgres>,
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
impl Policy<Postgres, LoggingEvent, LoggingError> for LoggingPolicy {
    async fn handle_event(&self, event: &StoreEvent<LoggingEvent>, _: &Pool<Postgres>) -> Result<(), LoggingError> {
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

// A store of events
pub type LogStore = PgStore<LoggingEvent, LoggingError>;

pub struct LoggingAggregate {
    event_store: LogStore,
}

impl LoggingAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, LoggingError> {
        Ok(Self {
            event_store: Self::new_store(pool).await?,
        })
    }

    async fn new_store(pool: &Pool<Postgres>) -> Result<LogStore, LoggingError> {
        let projectors: Vec<Box<dyn Projector<Postgres, LoggingEvent, LoggingError> + Send + Sync>> =
            vec![Box::new(LoggingProjector)];

        let policies: Vec<Box<dyn Policy<Postgres, LoggingEvent, LoggingError> + Send + Sync>> =
            vec![Box::new(LoggingPolicy)];

        PgStore::new::<Self>(pool, projectors, policies).await
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
            Self::Command::Log(msg) => Ok(vec![Self::Event::Logged(msg)]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // This aggregate state just counts the number of applied - equivalent to the number in the event store
        state + 1
    }
}

impl AggregateManager for LoggingAggregate {
    type EventStore = LogStore;

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }
}
