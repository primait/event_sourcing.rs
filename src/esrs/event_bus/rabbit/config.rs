use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ConnectionProperties, ExchangeKind};
use typed_builder::TypedBuilder;

use crate::event_bus::rabbit::error::RabbitEventBusError;

#[derive(TypedBuilder)]
pub struct RabbitEventBusConfig<'a> {
    pub(crate) url: &'a str,
    pub(crate) exchange: &'a str,
    #[builder(default)]
    pub(crate) connection_properties: ConnectionProperties,
    pub(crate) exchange_kind: ExchangeKind,
    #[builder(default)]
    pub(crate) exchange_options: ExchangeDeclareOptions,
    #[builder(default)]
    pub(crate) arguments: FieldTable,
    #[builder(default)]
    pub(crate) publish_routing_key: Option<String>,
    #[builder(default)]
    pub(crate) publish_options: BasicPublishOptions,
    #[builder(default)]
    pub(crate) publish_properties: BasicProperties,
    #[builder(default = Box::new(| _ | ()))]
    pub(crate) error_handler: Box<dyn Fn(RabbitEventBusError) + Send + Sync>,
}
