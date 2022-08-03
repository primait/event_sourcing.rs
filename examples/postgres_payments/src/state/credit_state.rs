#[derive(Debug, Clone)]
pub struct CreditCardState {
    pub total_amount: i32,
    pub ceiling: i32,
}

impl Default for CreditCardState {
    fn default() -> Self {
        Self {
            total_amount: 0,
            ceiling: 1500,
        }
    }
}

impl CreditCardState {
    pub const fn add_amount(self, amount: i32) -> Self {
        Self {
            total_amount: self.total_amount + amount,
            ceiling: self.ceiling,
        }
    }

    pub const fn sub_amount(self, amount: i32) -> Self {
        Self {
            total_amount: self.total_amount - amount,
            ceiling: self.ceiling,
        }
    }
}
