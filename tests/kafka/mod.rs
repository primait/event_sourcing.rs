use chrono::Utc;
use uuid::Uuid;

use esrs::event_bus::kafka::{KafkaEventBus, KafkaEventBusConfig};
use esrs::event_bus::EventBus;
use esrs::StoreEvent;

use crate::aggregate::{TestAggregate, TestEvent};

#[tokio::test]
async fn kafka_event_bus_test() {
    let kafka_broker_url: String = std::env::var("KAFKA_BROKERS_URL").unwrap();

    let config: KafkaEventBusConfig = KafkaEventBusConfig::builder()
        .broker_url_list(kafka_broker_url.as_str())
        .topic("test")
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
}

fn consumer() {}
