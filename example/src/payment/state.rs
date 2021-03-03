#[derive(Clone)]
pub struct PaymentState {
    pub total_amount: f32,
}

impl Default for PaymentState {
    fn default() -> Self {
        Self { total_amount: 0.0 }
    }
}

impl PaymentState {
    pub fn add_amount(&mut self, amount: f32) -> &Self {
        self.total_amount += amount;
        self
    }

    pub fn sub_amount(&mut self, amount: f32) -> &Self {
        self.total_amount -= amount;
        self
    }
}
