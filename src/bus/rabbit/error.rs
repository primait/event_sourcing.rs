/// The `RabbitError` enum defines the following error types:
///
/// - `Json`: Indicates a failure in serializing/deserializing the event payload.
/// - `Rabbit`: Indicates an error occurred while establishing a connection with the RabbitMQ server
///             or an error encountered during the event publishing process.
/// - `PublishNack`: Indicates an error encountered during the publishing process, indicating the
///                  server responding with a `Nack`.
#[derive(Debug)]
pub enum RabbitEventBusError {
    Json(serde_json::Error),
    Rabbit(lapin::Error),
    PublishNack,
}

impl From<serde_json::Error> for RabbitEventBusError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<lapin::Error> for RabbitEventBusError {
    fn from(value: lapin::Error) -> Self {
        Self::Rabbit(value)
    }
}
