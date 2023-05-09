use std::marker::PhantomData;

use async_trait::async_trait;
use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::publisher_confirm::Confirmation;
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind};
use serde::Serialize;

pub use builder::RabbitEventBusBuilder;

use crate::esrs::event_bus::EventBus;
use crate::{Aggregate, StoreEvent};

mod builder;

pub struct RabbitEventBus<A> {
    channel: Channel,
    exchange: String,
    publish_routing_key: String,
    publish_options: BasicPublishOptions,
    publish_properties: BasicProperties,
    error_handler: Box<dyn Fn(RabbitEventBusError) + Sync>,
    _phantom: PhantomData<A>,
}

impl<A> RabbitEventBus<A>
where
    A: Aggregate,
{
    #[allow(clippy::too_many_arguments)]
    pub(crate) async fn new(
        url: &str,
        exchange: &str,
        connection_properties: ConnectionProperties,
        exchange_kind: ExchangeKind,
        options: ExchangeDeclareOptions,
        arguments: FieldTable,
        publish_routing_key: String,
        publish_options: BasicPublishOptions,
        publish_properties: BasicProperties,
    ) -> Result<Self, lapin::Error> {
        let connection: Connection = Connection::connect(url, connection_properties).await?;
        let channel: Channel = connection.create_channel().await?;

        channel
            .exchange_declare(exchange, exchange_kind, options, arguments)
            .await?;

        Ok(Self {
            channel,
            exchange: exchange.to_string(),
            publish_routing_key,
            publish_options,
            publish_properties,
            error_handler: Box::new(|_| ()),
            _phantom: PhantomData::default(),
        })
    }
}

#[async_trait]
impl<A> EventBus<A> for RabbitEventBus<A>
where
    Self: Send,
    A: Aggregate + Send + Sync,
    A::Event: Serialize,
{
    async fn publish(&self, store_event: &StoreEvent<A::Event>) {
        if let Err(error) = publish(self, store_event).await {
            (self.error_handler)(error)
        }
    }
}

async fn publish<A>(reb: &RabbitEventBus<A>, store_event: &StoreEvent<A::Event>) -> Result<(), RabbitEventBusError>
where
    A: Aggregate + Send + Sync,
    A::Event: Serialize,
{
    let bytes: Vec<u8> = serde_json::to_vec(store_event)?;

    let confirmation: Confirmation = reb
        .channel
        .basic_publish(
            reb.exchange.as_str(),
            reb.publish_routing_key.as_str(),
            reb.publish_options,
            &bytes,
            reb.publish_properties.clone(),
        )
        .await?
        .await?;

    match confirmation {
        Confirmation::Ack(_) => Ok(()),
        Confirmation::Nack(_) => Err(RabbitEventBusError::RabbitNack),
        Confirmation::NotRequested => Err(RabbitEventBusError::RabbitNotRequested),
    }
}

pub enum RabbitEventBusError {
    Json(serde_json::Error),
    Rabbit(lapin::Error),
    RabbitNack,
    RabbitNotRequested,
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
