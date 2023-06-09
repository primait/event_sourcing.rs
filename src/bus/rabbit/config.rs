use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ConnectionProperties, ExchangeKind};
use typed_builder::TypedBuilder;

use crate::bus::rabbit::error::RabbitEventBusError;

#[derive(TypedBuilder)]
pub struct RabbitEventBusConfig<'a> {
    /// The connection string for the RabbitMQ server, including the protocol, host, port, and
    /// virtual host.
    pub(crate) url: &'a str,
    /// The name of the RabbitMQ exchange to use for publishing events. If not specified, events will
    /// be published to the default exchange.
    pub(crate) exchange: &'a str,
    /// Additional connection properties.
    #[builder(default)]
    pub(crate) connection_properties: ConnectionProperties,
    /// The type of the RabbitMQ exchange, such as "direct", "topic", "fanout", etc.
    /// Defaults to "direct" if not specified.
    pub(crate) exchange_kind: ExchangeKind,
    /// Additional exchange options.
    #[builder(default)]
    pub(crate) exchange_options: ExchangeDeclareOptions,
    /// Additional exchange arguments.
    #[builder(default)]
    pub(crate) exchange_arguments: FieldTable,
    /// Optional routing key configuration. The routing key is a string value attached to each message
    /// during the publishing process. When a message arrives at the exchange, it examines the
    /// routing key and compares it to the routing rules specified in the bindings.
    #[builder(default)]
    pub(crate) publish_routing_key: Option<String>,
    /// Additional publish options.
    #[builder(default)]
    pub(crate) publish_options: BasicPublishOptions,
    /// Additional publish properties.
    #[builder(default)]
    pub(crate) publish_properties: BasicProperties,
    /// A boxed anonymous function utilized to provide a form of error handling, commonly used for
    /// reporting purposes.
    #[builder(default = Box::new(| _ | ()))]
    pub(crate) error_handler: Box<dyn Fn(RabbitEventBusError) + Send + Sync>,
}
