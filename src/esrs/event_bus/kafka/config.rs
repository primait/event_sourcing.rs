use std::time::Duration;

use rdkafka::ClientConfig;
use typed_builder::TypedBuilder;

use crate::event_bus::kafka::error::KafkaEventBusError;
use crate::event_bus::kafka::KafkaEventBus;
use crate::Aggregate;

#[derive(TypedBuilder)]
pub struct KafkaEventBusConfig<'a> {
    broker_url_list: &'a str,
    topic: String,
    #[builder(default, setter(strip_option))]
    security: Option<Security<'a>>,
    #[builder(default = 5000)]
    request_timeout: u64,
    #[builder(default, setter(strip_option))]
    client_config: Option<ClientConfig>,
    #[builder(default = Box::new(|_| ()))]
    error_handler: Box<dyn Fn(KafkaEventBusError) + Sync>,
}

pub struct Security<'a> {
    username: &'a str,
    password: &'a str,
    sasl_mechanism: &'a str,
}

impl KafkaEventBusConfig<'_> {
    pub async fn into_kafka_event_bus<A>(self) -> Result<KafkaEventBus<A>, rdkafka::error::KafkaError>
    where
        A: Aggregate,
    {
        let mut config: ClientConfig = self.client_config.unwrap_or_default();
        config
            .set("metadata.broker.list", self.broker_url_list)
            .set("request.timeout.ms", self.request_timeout.to_string());

        if let Some(security) = self.security {
            config
                .set("security.protocol", "SASL_SSL")
                .set("sasl.mechanisms", security.sasl_mechanism)
                .set("sasl.username", security.username)
                .set("sasl.password", security.password);
        }

        KafkaEventBus::new(
            self.topic,
            Duration::from_millis(self.request_timeout),
            config,
            self.error_handler,
        )
    }
}
