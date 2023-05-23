use rdkafka::consumer::{Consumer, StreamConsumer};
use rdkafka::{ClientConfig, Message};
use serde::de::DeserializeOwned;

use esrs::{Aggregate, EventHandler, StoreEvent};

pub struct KafkaEventBusConsumer<A> {
    consumer: StreamConsumer,
    event_handlers: Vec<Box<dyn EventHandler<A> + Send>>,
}

impl<A> KafkaEventBusConsumer<A>
where
    A: Aggregate,
    A::Event: DeserializeOwned,
{
    pub fn new(broker_urls: &str, topic: &str, event_handlers: Vec<Box<dyn EventHandler<A> + Send>>) -> Self {
        // In a separate thread
        let consumer: StreamConsumer = ClientConfig::new()
            .set("group.id", "1")
            .set("bootstrap.servers", broker_urls)
            .set("enable.partition.eof", "false")
            .set("session.timeout.ms", "6000")
            .set("max.poll.interval.ms", "6000")
            .set("enable.auto.commit", "true")
            .set("allow.auto.create.topics", "true")
            .set("auto.offset.reset", "earliest")
            .create()
            .expect("Consumer creation failed");

        consumer
            .subscribe(&[topic])
            .expect("Can't subscribe to specified topics");

        Self {
            consumer,
            event_handlers,
        }
    }

    pub async fn handle(&self) {
        let message = self.consumer.recv().await.unwrap();
        let data = message.payload().unwrap();
        let store_event: StoreEvent<A::Event> = serde_json::from_slice(data).unwrap();

        for event_handler in &self.event_handlers {
            event_handler.handle(&store_event).await;
        }
    }
}
