use std::marker::PhantomData;

use async_trait::async_trait;
use lapin::options::BasicPublishOptions;
use lapin::publisher_confirm::Confirmation;
use lapin::{BasicProperties, Channel, Connection};
use serde::Serialize;

pub use config::RabbitEventBusConfig;
pub use error::RabbitEventBusError;

use crate::esrs::event_bus::EventBus;
use crate::{Aggregate, StoreEvent};

mod config;
mod error;

pub struct RabbitEventBus<A> {
    channel: Channel,
    exchange: String,
    publish_routing_key: Option<String>,
    publish_options: BasicPublishOptions,
    publish_properties: BasicProperties,
    error_handler: Box<dyn Fn(RabbitEventBusError) + Send + Sync>,
    _phantom: PhantomData<A>,
}

impl<A> RabbitEventBus<A>
where
    A: Aggregate,
{
    pub async fn new(config: RabbitEventBusConfig<'_>) -> Result<RabbitEventBus<A>, RabbitEventBusError> {
        let connection: Connection = Connection::connect(config.url, config.connection_properties).await?;
        let channel: Channel = connection.create_channel().await?;

        channel
            .exchange_declare(
                config.exchange,
                config.exchange_kind,
                config.exchange_options,
                config.arguments,
            )
            .await?;

        Ok(Self {
            channel,
            exchange: config.exchange.to_string(),
            publish_routing_key: config.publish_routing_key,
            publish_options: config.publish_options,
            publish_properties: config.publish_properties,
            error_handler: config.error_handler,
            _phantom: PhantomData::default(),
        })
    }
}

#[async_trait]
impl<A> EventBus<A> for RabbitEventBus<A>
where
    Self: Send,
    A: Aggregate + Send + Sync,
    A::Event: Serialize + Sync,
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
    let routing_key: String = reb.publish_routing_key.clone().unwrap_or_default();

    let confirmation: Confirmation = reb
        .channel
        .basic_publish(
            reb.exchange.as_str(),
            routing_key.as_str(),
            reb.publish_options,
            &bytes,
            reb.publish_properties.clone(),
        )
        .await?
        .await?;

    match confirmation {
        Confirmation::Ack(_) => Ok(()),
        Confirmation::NotRequested => Ok(()),
        Confirmation::Nack(_) => Err(RabbitEventBusError::PublishNack),
    }
}
