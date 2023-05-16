use chrono::Utc;
use rdkafka::admin::{AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::FromClientConfig;
use rdkafka::consumer::{CommitMode, Consumer, StreamConsumer};
use rdkafka::ClientConfig;
use uuid::Uuid;

use esrs::event_bus::kafka::{KafkaEventBus, KafkaEventBusConfig};
use esrs::event_bus::EventBus;
use esrs::StoreEvent;

use crate::aggregate::{TestAggregate, TestEvent};

#[tokio::test]
async fn kafka_event_bus_test() {
    let kafka_broker_url: String = std::env::var("KAFKA_BROKERS_URL").unwrap();
    let topic: &str = "test";

    create_topic(kafka_broker_url.as_str(), topic).await;

    let config: KafkaEventBusConfig = KafkaEventBusConfig::builder()
        .broker_url_list(kafka_broker_url.as_str())
        .topic(topic)
        .error_handler(Box::new(|error| panic!("{:?}", error)))
        .build();

    let bus: KafkaEventBus<TestAggregate> = match KafkaEventBus::new(config).await {
        Ok(bus) => bus,
        Err(error) => panic!("{:?}", error),
    };

    let store_event: StoreEvent<TestEvent> = StoreEvent {
        id: Uuid::new_v4(),
        aggregate_id: Uuid::new_v4(),
        payload: TestEvent { add: 1 },
        occurred_on: Utc::now(),
        sequence_number: 1,
    };

    bus.publish(&store_event).await;

    let consumer = consumer(kafka_broker_url.as_str(), topic);

    match consumer.recv().await {
        Err(e) => panic!("Kafka error: {}", e),
        Ok(m) => {
            consumer.commit_message(&m, CommitMode::Async).unwrap();
        }
    };
}

fn consumer(broker_urls: &str, topic: &str) -> StreamConsumer {
    let consumer: StreamConsumer = ClientConfig::new()
        .set("group.id", "1")
        .set("bootstrap.servers", broker_urls)
        .set("enable.partition.eof", "false")
        .set("session.timeout.ms", "6000")
        .set("max.poll.interval.ms", "6000")
        .set("enable.auto.commit", "true")
        .set("allow.auto.create.topics", "true")
        .create()
        .expect("Consumer creation failed");

    consumer
        .subscribe(&[topic])
        .expect("Can't subscribe to specified topics");

    consumer
}

async fn create_topic(broker_urls: &str, topic: &str) {
    let mut admin_config = ClientConfig::new();
    admin_config.set("bootstrap.servers", broker_urls);

    let admin_client = rdkafka::admin::AdminClient::from_config(&admin_config).unwrap();

    let new_topic = NewTopic::new(topic, 1, TopicReplication::Fixed(1));

    let _ = admin_client
        .create_topics(&[new_topic], &AdminOptions::default())
        .await
        .unwrap();
}
