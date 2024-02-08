use futures::TryStreamExt;
use lapin::options::{BasicConsumeOptions, QueueBindOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties, Consumer};
use serde::de::DeserializeOwned;

use esrs::handler::EventHandler;
use esrs::store::StoreEvent;
use esrs::Aggregate;

use crate::common::util::random_letters;

pub struct RabbitEventBusConsumer<A> {
    consumer: Consumer,
    event_handlers: Vec<Box<dyn EventHandler<A> + Send>>,
}

impl<A> RabbitEventBusConsumer<A>
where
    A: Aggregate,
    A::Event: DeserializeOwned,
{
    pub async fn new(rabbit_url: &str, exchange: &str, event_handlers: Vec<Box<dyn EventHandler<A> + Send>>) -> Self {
        let queue = format!("{}_bus_queue", random_letters());

        let conn = Connection::connect(rabbit_url, ConnectionProperties::default())
            .await
            .unwrap();

        let channel = conn.create_channel().await.unwrap();

        channel
            .queue_declare(queue.as_str(), QueueDeclareOptions::default(), FieldTable::default())
            .await
            .unwrap();

        channel
            .queue_bind(
                queue.as_str(),
                exchange,
                "",
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await
            .unwrap();

        let consume_opts = BasicConsumeOptions {
            no_local: false,
            no_ack: true,
            exclusive: false,
            nowait: false,
        };

        let consumer = channel
            .basic_consume(
                queue.as_str(),
                "rabbit_event_bus_consumer",
                consume_opts,
                FieldTable::default(),
            )
            .await
            .unwrap();

        Self {
            consumer,
            event_handlers,
        }
    }

    pub async fn handle(&mut self) {
        let message = self.consumer.try_next().await.unwrap().unwrap();
        let store_event: StoreEvent<A::Event> = serde_json::from_slice(&message.data).unwrap();

        for event_handler in &self.event_handlers {
            event_handler.handle(&store_event).await;
        }
    }
}
