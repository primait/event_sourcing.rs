use rdkafka::ClientConfig;
use typed_builder::TypedBuilder;

use crate::event_bus::kafka::error::KafkaEventBusError;

#[derive(TypedBuilder)]
pub struct KafkaEventBusConfig<'a> {
    pub(crate) broker_url_list: &'a str,
    pub(crate) topic: &'a str,
    #[builder(default, setter(strip_option))]
    pub(crate) security: Option<Security<'a>>,
    #[builder(default = 5000)]
    pub(crate) request_timeout: u64,
    #[builder(default, setter(strip_option))]
    pub(crate) client_config: Option<ClientConfig>,
    #[builder(default = Box::new(|_| ()))]
    pub(crate) error_handler: Box<dyn Fn(KafkaEventBusError) + Send + Sync>,
}

pub struct Security<'a> {
    pub(crate) username: &'a str,
    pub(crate) password: &'a str,
    pub(crate) sasl_mechanism: &'a str,
}
