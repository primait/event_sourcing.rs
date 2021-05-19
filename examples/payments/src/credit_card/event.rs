use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum CreditCardEvent {
    Payed { amount: i32 },
    Refunded { amount: i32 },
}
