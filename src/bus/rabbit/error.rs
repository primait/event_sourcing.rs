/// The `RabbitError` enum defines the following error types:
///
/// - `Json`: Indicates a failure in serializing/deserializing the event payload.
/// - `Rabbit`: Indicates an error occurred while establishing a connection with the RabbitMQ server
///             or an error encountered during the event publishing process.
/// - `PublishNack`: Indicates an error encountered during the publishing process, indicating the
///                  server responding with a `Nack`.
#[derive(thiserror::Error, Debug)]
pub enum RabbitEventBusError {
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Rabbit(#[from] lapin::Error),
    #[error("Received nack on publish")]
    PublishNack,
}
