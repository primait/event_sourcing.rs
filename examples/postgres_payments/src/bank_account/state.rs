#[derive(Debug, Clone)]
pub struct BankAccountState {
    pub balance: i32,
}

impl Default for BankAccountState {
    fn default() -> Self {
        Self { balance: 0 }
    }
}

impl BankAccountState {
    pub fn add_amount(self, amount: i32) -> Self {
        Self {
            balance: self.balance + amount,
        }
    }

    pub fn sub_amount(self, amount: i32) -> Self {
        Self {
            balance: self.balance - amount,
        }
    }
}
