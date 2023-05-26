use std::time::Duration;

use lapin::ExchangeKind;
use rdkafka::admin::{AdminOptions, NewTopic, TopicReplication};
use rdkafka::config::FromClientConfig;
use rdkafka::ClientConfig;
use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::event_bus::kafka::{KafkaEventBus, KafkaEventBusConfig};
use esrs::event_bus::rabbit::{RabbitEventBus, RabbitEventBusConfig};
use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState};

use crate::common::{new_pool, random_letters, BasicAggregate, BasicCommand, BasicEventHandler, BasicView};
use crate::kafka::KafkaEventBusConsumer;
use crate::rabbit::RabbitEventBusConsumer;

#[path = "../common/lib.rs"]
mod common;
mod kafka;
mod rabbit;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let rabbit_url: String = std::env::var("RABBIT_URL").unwrap();
    let kafka_broker_url: String = std::env::var("KAFKA_BROKERS_URL").unwrap();

    let topic_and_exchange: String = format!("{}_test", random_letters());

    create_topic(kafka_broker_url.as_str(), topic_and_exchange.as_str()).await;

    let config: RabbitEventBusConfig = RabbitEventBusConfig::builder()
        .url(rabbit_url.as_str())
        .exchange(topic_and_exchange.as_str())
        .exchange_kind(ExchangeKind::Direct)
        .build();
    let rabbit_event_bus: RabbitEventBus<BasicAggregate> = RabbitEventBus::new(config).await.unwrap();

    let config: KafkaEventBusConfig = KafkaEventBusConfig::builder()
        .broker_url_list(kafka_broker_url.as_str())
        .topic(topic_and_exchange.as_str())
        .build();
    let kafka_event_bus: KafkaEventBus<BasicAggregate> = KafkaEventBus::new(config).await.unwrap();

    // This is a multithread code block. In the target application this should live somewhere
    // separated from the main code. It can live in another thread, in another process or in another
    // machine.
    ///////////////////////////////
    // In one thread
    let rabbit_view: BasicView = BasicView::new("rabbit_view", &pool).await;
    let rabbit_event_handler: BasicEventHandler = BasicEventHandler {
        pool: pool.clone(),
        view: rabbit_view.clone(),
    };

    let mut rabbit_consumer = RabbitEventBusConsumer::new(
        rabbit_url.as_str(),
        topic_and_exchange.as_str(),
        vec![Box::new(rabbit_event_handler)],
    )
    .await;

    let rabbit_join_handle = tokio::spawn(tokio::time::timeout(Duration::from_secs(5), async move {
        rabbit_consumer.handle().await;
    }));

    // In another thread
    let kafka_view: BasicView = BasicView::new("kafka_view", &pool).await;
    let kafka_event_handler: BasicEventHandler = BasicEventHandler {
        pool: pool.clone(),
        view: kafka_view.clone(),
    };

    let kafka_consumer = KafkaEventBusConsumer::new(
        kafka_broker_url.as_str(),
        topic_and_exchange.as_str(),
        vec![Box::new(kafka_event_handler)],
    );

    let kafka_join_handle = tokio::spawn(tokio::time::timeout(Duration::from_secs(5), async move {
        kafka_consumer.handle().await
    }));
    ///////////////////////////////

    let store: PgStore<BasicAggregate> = PgStoreBuilder::new(pool.clone())
        .with_event_buses(vec![Box::new(rabbit_event_bus), Box::new(kafka_event_bus)])
        .try_build()
        .await
        .unwrap();

    let manager: AggregateManager<PgStore<BasicAggregate>> = AggregateManager::new(store);

    let content: &str = "view row content";
    let aggregate_state: AggregateState<()> = AggregateState::default();
    let aggregate_id: Uuid = *aggregate_state.id();
    let command = BasicCommand {
        content: content.to_string(),
    };
    manager.handle_command(aggregate_state, command).await.unwrap();

    let (rabbit_timeout_result, kafka_timeout_result) = tokio::join!(rabbit_join_handle, kafka_join_handle);

    assert!(rabbit_timeout_result.is_ok());
    assert!(kafka_timeout_result.is_ok());

    let kafka_view_row = kafka_view.by_id(aggregate_id, &pool).await.unwrap();
    assert!(kafka_view_row.is_some());
    assert_eq!(kafka_view_row.unwrap().content, content);

    let rabbit_view_row = rabbit_view.by_id(aggregate_id, &pool).await.unwrap();
    assert!(rabbit_view_row.is_some());
    assert_eq!(rabbit_view_row.unwrap().content, content);
}

/// For kafka is required to create the topic
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
