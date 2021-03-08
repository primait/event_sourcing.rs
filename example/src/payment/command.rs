pub enum PaymentCommand {
    Pay { amount: i32 },
    Refund { amount: i32 },
}
