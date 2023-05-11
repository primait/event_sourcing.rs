use std::marker::PhantomData;
use std::time::Duration;

use async_trait::async_trait;
use rdkafka::producer::{FutureProducer, FutureRecord};
use rdkafka::ClientConfig;
use serde::Serialize;

pub use config::KafkaEventBusConfig;

use crate::event_bus::kafka::error::KafkaEventBusError;
use crate::event_bus::EventBus;
use crate::{Aggregate, StoreEvent};

mod config;
mod error;

pub struct KafkaEventBus<A> {
    producer: FutureProducer,
    topic: String,
    request_timeout: Duration,
    error_handler: Box<dyn Fn(KafkaEventBusError) + Sync>,
    _phantom: PhantomData<A>,
}

impl<A> KafkaEventBus<A> {
    pub(crate) fn new(
        topic: String,
        queue_timeout: Duration,
        config: ClientConfig,
        error_handler: Box<dyn Fn(KafkaEventBusError) + Sync>,
    ) -> Result<Self, rdkafka::error::KafkaError> {
        Ok(Self {
            producer: config.create()?,
            topic,
            request_timeout: queue_timeout,
            error_handler,
            _phantom: Default::default(),
        })
    }
}

#[async_trait]
impl<A> EventBus<A> for KafkaEventBus<A>
where
    Self: Send,
    A: Aggregate + Send + Sync,
    A::Event: Serialize,
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
    let bytes: Vec<u8> = serde_json::to_vec(store_event)?;

    let _ = event_bus
        .producer
        .send(
            FutureRecord::<String, Vec<u8>>::to(event_bus.topic.as_str()).payload(&bytes),
            event_bus.request_timeout,
        )
        .await?;

    Ok(())
}
