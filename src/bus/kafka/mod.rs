use std::marker::PhantomData;
use std::time::Duration;

use async_trait::async_trait;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use serde::Serialize;

pub use config::KafkaEventBusConfig;
pub use error::KafkaEventBusError;

use crate::bus::EventBus;
use crate::store::StoreEvent;
use crate::Aggregate;

mod config;
mod error;

/// The [`KafkaEventBus`] provides an implementation of the `EventBus` trait for publishing events
/// using Apache Kafka as the underlying messaging system.
pub struct KafkaEventBus<A> {
    producer: FutureProducer,
    topic: String,
    request_timeout: Duration,
    error_handler: Box<dyn Fn(KafkaEventBusError) + Send + Sync>,
    _phantom: PhantomData<A>,
}

impl<A> KafkaEventBus<A> {
    pub async fn new(config: KafkaEventBusConfig<'_>) -> Result<KafkaEventBus<A>, KafkaEventBusError> {
        let mut client_config: ClientConfig = config.client_config.unwrap_or_default();
        client_config
            .set("metadata.broker.list", config.broker_url_list)
            .set("request.timeout.ms", config.request_timeout.to_string());

        if let Some(security) = config.security {
            client_config
                .set("security.protocol", "SASL_SSL")
                .set("sasl.mechanisms", security.sasl_mechanism)
                .set("sasl.username", security.username)
                .set("sasl.password", security.password);
        }

        Ok(Self {
            producer: client_config.create()?,
            topic: config.topic.to_string(),
            request_timeout: Duration::from_millis(config.request_timeout),
            error_handler: config.error_handler,
            _phantom: Default::default(),
        })
    }
}

#[async_trait]
impl<A> EventBus<A> for KafkaEventBus<A>
where
    Self: Send,
    A: Aggregate + Send + Sync,
    A::Event: Serialize + Sync,
{
    async fn publish(&self, store_event: &StoreEvent<A::Event>) {
        match publish(self, store_event).await {
            Ok(_) => (),
            Err(err) => (self.error_handler)(err),
        }
    }
}

async fn publish<A>(event_bus: &KafkaEventBus<A>, store_event: &StoreEvent<A::Event>) -> Result<(), KafkaEventBusError>
where
    A: Aggregate + Send + Sync,
    A::Event: Serialize,
{
    let key: String = store_event.aggregate_id.to_string();
    let bytes: Vec<u8> = serde_json::to_vec(store_event)?;

    let _ = event_bus
        .producer
        .send(
            FutureRecord::<String, Vec<u8>>::to(event_bus.topic.as_str())
                .key(&key)
                .payload(&bytes),
            event_bus.request_timeout,
        )
        .await?;

    Ok(())
}
