pub enum BankAccountCommand {
    Withdraw { amount: i32 },
    Deposit { amount: i32 },
}
