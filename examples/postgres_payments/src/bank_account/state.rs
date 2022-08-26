#[derive(Default, Debug, Clone)]
pub struct BankAccountState {
    pub balance: i32,
}

impl BankAccountState {
    pub const fn add_amount(self, amount: i32) -> Self {
        Self {
            balance: self.balance + amount,
        }
    }

    pub const fn sub_amount(self, amount: i32) -> Self {
        Self {
            balance: self.balance - amount,
        }
    }
}
