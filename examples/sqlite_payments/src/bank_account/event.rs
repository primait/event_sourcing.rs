use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BankAccountEvent {
    Withdrawn { amount: i32 },
    Deposited { amount: i32 },
}
