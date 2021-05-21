pub enum CreditCardCommand {
    Pay { amount: i32 },
    Refund { amount: i32 },
}
