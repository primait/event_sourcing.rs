use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use uuid::Uuid;

use esrs::{Aggregate, AggregateManager, AggregateState, EventHandler, StoreEvent};

#[derive(Clone)]
pub struct LoggingAggregate;

impl Aggregate for LoggingAggregate {
    const NAME: &'static str = "message";
    type State = u64;
    type Command = LoggingCommand;
    type Event = LoggingEvent;
    type Error = LoggingError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
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

// A very simply policy, which tries to log an event, and creates another command after it finishes
// which indicates success or failure to log

struct LoggingEventHandler {
    // It's possible here to keep an AggregateManager instance or an EventStore instance too.
    manager: AggregateManager<LoggingAggregate>,
}

#[async_trait]
impl EventHandler<LoggingAggregate> for LoggingEventHandler {
    async fn handle(&self, event: &StoreEvent<LoggingEvent>) {
        let aggregate_id: Uuid = event.aggregate_id;

        let aggregate_state: AggregateState<u64> = match self.manager.load(aggregate_id).await {
            Ok(Some(aggregate_state)) => aggregate_state,
            Ok(None) => AggregateState::with_id(aggregate_id),
            Err(e) => return println!("Error loading aggregate {}", e),
        }; // This should never happen

        if let LoggingEvent::Received(msg) = event.payload() {
            if msg.contains("fail_policy") {
                match self.manager.handle_command(aggregate_state, LoggingCommand::Fail).await {
                    Ok(_) => (),
                    Err(e) => println!("Error handling command {}", e),
                };
                return;
            }
            println!("Logged via policy from {}: {}", aggregate_id, msg);
            match self
                .manager
                .handle_command(aggregate_state, LoggingCommand::Succeed)
                .await
            {
                Ok(_) => (),
                Err(e) => println!("Error handling command {}", e),
            };
        };
    }
}

// A simple error enum for event processing errors
#[derive(Debug, Error)]
pub enum LoggingError {
    #[error("Err {0}")]
    Domain(String),

    #[error(transparent)]
    Json(#[from] esrs::error::JsonError),

    #[error(transparent)]
    Sql(#[from] esrs::error::SqlxError),
}

// The events to be processed. On receipt of a new log message, the aggregate stores a Received event.
// If the message contained within can be logged, it is, and then a Succeeded event is produced, otherwise
// a Failed event is produced.
#[derive(Serialize, Deserialize, Debug)]
pub enum LoggingEvent {
    Received(String),
    Succeeded,
    Failed,
}

// The aggregate receives commands to log a message
pub enum LoggingCommand {
    TryLog(String),
    Succeed,
    Fail,
}
