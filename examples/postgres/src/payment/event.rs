use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum PaymentEvent {
    Payed { amount: i32 },
    Refunded { amount: i32 },
}
