#[derive(Clone)]
pub struct PaymentState {
    pub total_amount: i32,
}

impl Default for PaymentState {
    fn default() -> Self {
        Self { total_amount: 0 }
    }
}

impl PaymentState {
    pub fn add_amount(self, amount: i32) -> Self {
        Self {
            total_amount: self.total_amount + amount,
        }
    }

    pub fn sub_amount(self, amount: i32) -> Self {
        Self {
            total_amount: self.total_amount - amount,
        }
    }
}
