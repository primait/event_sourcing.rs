use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum CreditCardEvent {
    Payed { amount: i32 },
    Refunded { amount: i32 },
}
