use rdkafka::ClientConfig;
use typed_builder::TypedBuilder;

use crate::bus::kafka::error::KafkaEventBusError;

#[derive(TypedBuilder)]
pub struct KafkaEventBusConfig<'a> {
    /// A list of Kafka broker addresses in the format `host:port`. Multiple addresses can be
    /// specified for broker redundancy.
    pub(crate) broker_url_list: &'a str,
    /// The default topic to use for publishing events. If not specified, events need to be
    /// explicitly published to a specific topic.
    pub(crate) topic: &'a str,
    /// An optional configuration to enable SASL security for authorizing an event bus to publish
    /// events to a specific topic.
    #[builder(default, setter(strip_option))]
    pub(crate) security: Option<Security<'a>>,
    /// The maximum time in milliseconds the broker will wait for acknowledgments from replicas
    /// before returning an error.
    #[builder(default = 5000)]
    pub(crate) request_timeout: u64,
    /// Additional Kafka client configuration.
    #[builder(default, setter(strip_option))]
    pub(crate) client_config: Option<ClientConfig>,
    /// A boxed anonymous function utilized to provide a form of error handling, commonly used for
    /// reporting purposes.
    #[builder(default = Box::new(| _ | ()))]
    pub(crate) error_handler: Box<dyn Fn(KafkaEventBusError) + Send + Sync>,
}

pub struct Security<'a> {
    pub(crate) username: &'a str,
    pub(crate) password: &'a str,
    pub(crate) sasl_mechanism: &'a str,
}
