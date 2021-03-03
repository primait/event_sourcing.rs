pub enum PaymentCommand {
    Pay { amount: f32 },
    Refund { amount: f32 },
}
