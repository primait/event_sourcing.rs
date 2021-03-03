use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PaymentEvent {
    Payed { amount: f32 },
    Refunded { amount: f32 },
}
