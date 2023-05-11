use rdkafka::error::KafkaError;
use rdkafka::message::OwnedMessage;
use serde_json::Error;

pub enum KafkaEventBusError {
    Json(Error),
    Kafka(KafkaError),
}

impl From<Error> for KafkaEventBusError {
    fn from(value: Error) -> Self {
        Self::Json(value)
    }
}

impl From<(KafkaError, OwnedMessage)> for KafkaEventBusError {
    fn from((error, _): (KafkaError, OwnedMessage)) -> Self {
        Self::Kafka(error)
    }
}
