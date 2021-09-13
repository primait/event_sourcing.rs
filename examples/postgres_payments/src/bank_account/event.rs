use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub enum BankAccountEvent {
    Withdrawn { amount: i32 },
    Deposited { amount: i32 },
}
