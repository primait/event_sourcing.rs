use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum BankAccountEvent {
    Withdrawn { amount: i32 },
    Deposited { amount: i32 },
}

#[derive(Serialize, Deserialize, Debug)]
pub enum CreditCardEvent {
    Payed { amount: i32 },
    Refunded { amount: i32 },
}
