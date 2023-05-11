pub enum RabbitEventBusError {
    Json(serde_json::Error),
    Rabbit(lapin::Error),
    RabbitNack,
    RabbitNotRequested,
}

impl From<serde_json::Error> for RabbitEventBusError {
    fn from(value: serde_json::Error) -> Self {
        Self::Json(value)
    }
}

impl From<lapin::Error> for RabbitEventBusError {
    fn from(value: lapin::Error) -> Self {
        Self::Rabbit(value)
    }
}
