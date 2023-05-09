use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::types::FieldTable;
use lapin::{BasicProperties, ConnectionProperties, ExchangeKind};
use typed_builder::TypedBuilder;

use crate::esrs::event_bus::rabbit::RabbitEventBus;
use crate::Aggregate;

#[derive(TypedBuilder)]
pub struct RabbitEventBusBuilder<'a> {
    url: &'a str,
    exchange: &'a str,
    #[builder(default)]
    connection_properties: ConnectionProperties,
    exchange_kind: ExchangeKind,
    #[builder(default)]
    options: ExchangeDeclareOptions,
    #[builder(default)]
    arguments: FieldTable,
    publish_routing_key: String,
    #[builder(default)]
    publish_options: BasicPublishOptions,
    #[builder(default)]
    publish_properties: BasicProperties,
}

impl RabbitEventBusBuilder<'_> {
    pub async fn into_rabbit_event_bus<A>(self) -> Result<RabbitEventBus<A>, lapin::Error>
    where
        A: Aggregate,
    {
        RabbitEventBus::new(
            self.url,
            self.exchange,
            self.connection_properties,
            self.exchange_kind,
            self.options,
            self.arguments,
            self.publish_routing_key,
            self.publish_options,
            self.publish_properties,
        )
        .await
    }
}
