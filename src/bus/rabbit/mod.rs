use std::marker::PhantomData;
use std::time::Duration;

use async_trait::async_trait;
use bb8::ManageConnection;
use lapin::options::{BasicPublishOptions, ExchangeDeclareOptions};
use lapin::publisher_confirm::Confirmation;
use lapin::types::FieldTable;
use lapin::{BasicProperties, Channel, Connection, ConnectionProperties, ExchangeKind};
use serde::Serialize;

pub use config::RabbitEventBusConfig;
pub use error::RabbitEventBusError;

use crate::bus::EventBus;
use crate::store::StoreEvent;
use crate::Aggregate;

mod config;
mod error;

pub struct RabbitConnectionManager {
    url: String,
    connection_properties: ConnectionProperties,
}

#[async_trait]
impl ManageConnection for RabbitConnectionManager {
    type Connection = Connection;
    type Error = lapin::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        Connection::connect(&self.url, self.connection_properties.to_owned()).await
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        if self.has_broken(conn) {
            return Err(lapin::Error::InvalidConnectionState(conn.status().state()));
        }
        Ok(())
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        !conn.status().connected()
    }
}

pub struct RabbitChannelManager {
    connection_pool: bb8::Pool<RabbitConnectionManager>,
    exchange: String,
    exchange_kind: ExchangeKind,
    exchange_options: ExchangeDeclareOptions,
    exchange_arguments: FieldTable,
}

#[async_trait]
impl ManageConnection for RabbitChannelManager {
    type Connection = Channel;
    type Error = lapin::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let connection = match self.connection_pool.get().await {
            Ok(connection) => connection,
            Err(e) => match e {
                bb8::RunError::User(e) => return Err(e),
                bb8::RunError::TimedOut => return Err(lapin::Error::InvalidChannelState(lapin::ChannelState::Closed)),
            },
        };
        let channel = connection.create_channel().await?;
        channel
            .exchange_declare(
                &self.exchange,
                self.exchange_kind.to_owned(),
                self.exchange_options,
                self.exchange_arguments.to_owned(),
            )
            .await?;
        Ok(channel)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        if self.has_broken(conn) {
            return Err(lapin::Error::InvalidChannelState(conn.status().state()));
        }
        Ok(())
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        !conn.status().connected()
    }
}

/// The [`RabbitEventBus`] provides an implementation of the `EventBus` trait for publishing events
/// using RabbitMQ as the underlying messaging system.
pub struct RabbitEventBus<A> {
    channel_pool: bb8::Pool<RabbitChannelManager>,
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
        let connection_manager = RabbitConnectionManager {
            url: config.url.to_string(),
            connection_properties: config.connection_properties,
        };

        let connection_pool = bb8::Pool::builder()
            .max_size(2)
            .max_lifetime(Some(Duration::from_secs(10 * 60)))
            .idle_timeout(Some(Duration::from_secs(5 * 60)))
            .min_idle(Some(1))
            .build(connection_manager)
            .await?;

        let channel_manager = RabbitChannelManager {
            connection_pool,
            exchange: config.exchange.to_string(),
            exchange_kind: config.exchange_kind,
            exchange_options: config.exchange_options,
            exchange_arguments: config.exchange_arguments,
        };

        let channel_pool = bb8::Pool::builder()
            .max_size(10)
            .max_lifetime(Some(Duration::from_secs(5 * 60)))
            .idle_timeout(Some(Duration::from_secs(60)))
            .min_idle(Some(1))
            .build(channel_manager)
            .await?;

        Ok(Self {
            channel_pool,
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

    let channel = reb.channel_pool.get().await?;
    let confirmation: Confirmation = channel
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
