pub enum BankAccountCommand {
    Withdraw { amount: i32 },
    Deposit { amount: i32 },
}

pub enum CreditCardCommand {
    Pay { amount: i32 },
    Refund { amount: i32 },
}
