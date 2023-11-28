use chrono::Utc;
use futures::TryStreamExt;
use lapin::options::{BasicAckOptions, BasicConsumeOptions, QueueBindOptions, QueueDeclareOptions};
use lapin::types::FieldTable;
use lapin::{Connection, ConnectionProperties, Consumer, ExchangeKind};
use rand::prelude::IteratorRandom;
use uuid::Uuid;

use esrs::bus::rabbit::{RabbitEventBus, RabbitEventBusConfig};
use esrs::bus::EventBus;
use esrs::store::StoreEvent;

use crate::aggregate::{TestAggregate, TestEvent};

#[tokio::test]
#[ntest::timeout(10000)]
async fn rabbit_event_bus_test() {
    let rabbit_url: String = std::env::var("RABBIT_URL").unwrap();

    let exchange: String = format!("{}_test_exchange", random_letters());
    let queue: String = format!("{}_test_queue", random_letters());
    let routing_key: String = format!("{}__test_routing_key", random_letters());

    let config: RabbitEventBusConfig = RabbitEventBusConfig::builder()
        .url(rabbit_url.as_str())
        .exchange(exchange.as_str())
        .exchange_kind(ExchangeKind::Fanout)
        .publish_routing_key(Some(routing_key.to_string()))
        .error_handler(Box::new(|error| panic!("{:?}", error)))
        .build();

    let bus: RabbitEventBus<TestAggregate> = match RabbitEventBus::new(config).await {
        Ok(bus) => bus,
        Err(error) => panic!("{:?}", error),
    };

    let mut consumer: Consumer = consumer(
        rabbit_url.as_str(),
        exchange.as_str(),
        queue.as_str(),
        routing_key.as_str(),
    )
    .await;

    let event_id: Uuid = Uuid::new_v4();
    let store_event: StoreEvent<TestEvent> = StoreEvent {
        id: event_id,
        aggregate_id: Uuid::new_v4(),
        payload: TestEvent { add: 1 },
        occurred_on: Utc::now(),
        sequence_number: 1,
        version: None,
    };

    bus.publish(&store_event).await;

    match consumer.try_next().await {
        Ok(delivery_opt) => {
            let delivery = delivery_opt.expect("error in consumer");
            delivery.ack(BasicAckOptions::default()).await.unwrap();
            let event = serde_json::from_slice::<StoreEvent<TestEvent>>(delivery.data.as_slice()).unwrap();
            assert_eq!(event.id, event_id);
        }
        Err(_) => {
            panic!("try next returned error");
        }
    }
}

async fn consumer(url: &str, exchange: &str, queue: &str, routing_key: &str) -> Consumer {
    let conn = Connection::connect(&url, ConnectionProperties::default())
        .await
        .unwrap();

    let channel = conn.create_channel().await.unwrap();

    channel
        .queue_declare(queue, QueueDeclareOptions::default(), FieldTable::default())
        .await
        .unwrap();

    channel
        .queue_bind(
            queue,
            exchange,
            routing_key,
            QueueBindOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap();

    channel
        .basic_consume(
            queue,
            "test_consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await
        .unwrap()
}

#[allow(dead_code)]
pub fn random_letters() -> String {
    let mut rng = rand::thread_rng();
    let chars: String = (0..6)
        .map(|_| (b'a'..=b'z').choose(&mut rng).unwrap() as char)
        .collect();
    chars
}
