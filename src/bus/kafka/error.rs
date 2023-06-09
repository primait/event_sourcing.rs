/// The `KafkaError` enum defines the following error types:
///
/// - `Json`: Indicates a failure in serializing/deserializing the event payload.
/// - `Kafka`: Indicates an error occurred while establishing a connection with the Kafka cluster or
///            an error encountered during the event publishing process.
#[derive(thiserror::Error, Debug)]
pub enum KafkaEventBusError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Kafka(#[from] rdkafka::error::KafkaError),
}

impl From<(rdkafka::error::KafkaError, rdkafka::message::OwnedMessage)> for KafkaEventBusError {
    fn from((error, _): (rdkafka::error::KafkaError, rdkafka::message::OwnedMessage)) -> Self {
        Self::Kafka(error)
    }
}
