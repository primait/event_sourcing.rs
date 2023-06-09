/// The `KafkaError` enum defines the following error types:
///
/// - `Json`: Indicates a failure in serializing/deserializing the event payload.
/// - `Kafka`: Indicates an error occurred while establishing a connection with the Kafka cluster or
///            an error encountered during the event publishing process.
#[derive(Debug)]
pub enum KafkaEventBusError {
    Json(serde_json::Error),
    Kafka(rdkafka::error::KafkaError),
}

impl From<serde_json::Error> for KafkaEventBusError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<rdkafka::error::KafkaError> for KafkaEventBusError {
    fn from(value: rdkafka::error::KafkaError) -> Self {
        Self::Kafka(value)
    }
}

impl From<(rdkafka::error::KafkaError, rdkafka::message::OwnedMessage)> for KafkaEventBusError {
    fn from((error, _): (rdkafka::error::KafkaError, rdkafka::message::OwnedMessage)) -> Self {
        Self::Kafka(error)
    }
}
