use uuid::Uuid;

use esrs::aggregate::{Aggregate, AggregateState};
use esrs::sqlx;
use esrs::sqlx::postgres::PgPoolOptions;
use esrs::sqlx::{Pool, Postgres};
use esrs::store::PgStore;
use payments::bank_account::aggregate::BankAccountAggregate;
use payments::bank_account::command::BankAccountCommand;
use payments::bank_account::error::BankAccountError;
use payments::bank_account::event::BankAccountEvent;
use payments::bank_account::state::BankAccountState;
use payments::bank_account::store::BankAccountStore;
use payments::credit_card::aggregate::CreditCardAggregate;
use payments::credit_card::command::CreditCardCommand;
use payments::credit_card::error::CreditCardError;
use payments::credit_card::event::CreditCardEvent;
use payments::credit_card::projector::CreditCard;
use payments::credit_card::state::CreditCardState;
use payments::credit_card::store::CreditCardStore;

#[actix_rt::test]
async fn credit_card_aggregate_and_projector_test() {
    let connection_string: &str = "postgres://postgres:postgres@postgres:5432/postgres";

    let pool: Pool<Postgres> = PgPoolOptions::new()
        .connect(connection_string)
        .await
        .expect("Failed to create pool");

    let () = sqlx::migrate!("examples/payments/migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    // CreditCard aggregate store
    let credit_card_store: PgStore<CreditCardEvent, CreditCardError> = CreditCardStore::new(&pool).await.unwrap();

    // CreditCard aggregate
    let credit_card_aggregate: CreditCardAggregate = CreditCardAggregate::new(credit_card_store);

    let bank_account_id: Uuid = Uuid::new_v4();
    // Bank account and credit card share the same aggregate_id
    let credit_card_state: AggregateState<CreditCardState> = AggregateState::new(bank_account_id);
    assert_eq!(credit_card_state.inner().total_amount, 0);

    let bank_account_state: AggregateState<BankAccountState> =
        AggregateState::new_with_state(bank_account_id, BankAccountState::default());
    let bank_account_store: PgStore<BankAccountEvent, BankAccountError> = BankAccountStore::new(&pool).await.unwrap();
    let bank_account_aggregate: BankAccountAggregate = BankAccountAggregate::new(bank_account_store);

    // Salary deposit (1000)
    let bank_account_state: AggregateState<BankAccountState> = bank_account_aggregate
        .handle_command(bank_account_state, BankAccountCommand::Deposit { amount: 1000 })
        .await
        .unwrap();

    assert_eq!(
        bank_account_state.inner().balance,
        bank_account_aggregate
            .load(*bank_account_state.id())
            .await
            .unwrap()
            .inner()
            .balance
    );

    // Cannot pay negative amount
    let result = credit_card_aggregate
        .handle_command(credit_card_state.clone(), CreditCardCommand::Pay { amount: -10 })
        .await;

    assert_eq!(
        result.err().unwrap().to_string(),
        CreditCardError::NegativeAmount.to_string()
    );

    // Cannot refund negative amount
    let result = credit_card_aggregate
        .handle_command(credit_card_state.clone(), CreditCardCommand::Refund { amount: -10 })
        .await;

    assert_eq!(
        result.err().unwrap().to_string(),
        CreditCardError::NegativeAmount.to_string()
    );

    // Credit card payment of 1000 euros
    let credit_card_state: AggregateState<CreditCardState> = credit_card_aggregate
        .handle_command(credit_card_state, CreditCardCommand::Pay { amount: 1000 })
        .await
        .unwrap();

    assert_eq!(credit_card_state.inner().total_amount, 1000);

    // Tryin to pay 250 euros. Not enough money in bank account
    let result = credit_card_aggregate
        .handle_command(credit_card_state.clone(), CreditCardCommand::Pay { amount: 250 })
        .await;

    // Payment is saved the same. Policies are not transactional
    assert_eq!(
        result.err().unwrap().to_string(),
        BankAccountError::NegativeBalance.to_string()
    );

    // Deposit of other 1000 euros
    let bank_account_state: AggregateState<BankAccountState> =
        bank_account_aggregate.load(bank_account_id).await.unwrap();

    let _bank_account_state: AggregateState<BankAccountState> = bank_account_aggregate
        .handle_command(bank_account_state, BankAccountCommand::Deposit { amount: 1000 })
        .await
        .unwrap();

    // Credit card payment of 250 euros. Total amount is 1500 (policies are not transactional..)
    let credit_card_state: AggregateState<CreditCardState> =
        credit_card_aggregate.load(*credit_card_state.id()).await.unwrap();
    let credit_card_state: AggregateState<CreditCardState> = credit_card_aggregate
        .handle_command(credit_card_state, CreditCardCommand::Pay { amount: 250 })
        .await
        .unwrap();

    assert_eq!(credit_card_state.inner().total_amount, 1500);

    // Cannot exceed plafond (1500)
    let result = credit_card_aggregate
        .handle_command(credit_card_state.clone(), CreditCardCommand::Pay { amount: 300 })
        .await;

    assert_eq!(
        result.err().unwrap().to_string(),
        CreditCardError::PlafondLimitReached.to_string()
    );

    // Refund of 250 euros. Total amount is 1250
    let credit_card_state: AggregateState<CreditCardState> = credit_card_aggregate
        .handle_command(credit_card_state, CreditCardCommand::Refund { amount: 250 })
        .await
        .unwrap();

    assert_eq!(credit_card_state.inner().total_amount, 1250);
    assert_eq!(
        credit_card_state.inner().total_amount,
        credit_card_aggregate
            .load(*credit_card_state.id())
            .await
            .unwrap()
            .inner()
            .total_amount
    );

    // 3 credit_card payments have been done
    let credit_card_payments: Vec<CreditCard> = CreditCard::by_credit_card_id(bank_account_id, &pool).await.unwrap();

    assert_eq!(credit_card_payments.len(), 4);
    assert!(credit_card_payments
        .iter()
        .any(|payment| payment.amount == 1000 && payment.credit_card_payment_type == "pay"));
    assert!(credit_card_payments
        .iter()
        .any(|payment| payment.amount == 250 && payment.credit_card_payment_type == "pay"));
    assert!(credit_card_payments
        .iter()
        .any(|payment| payment.amount == 250 && payment.credit_card_payment_type == "pay"));
    assert!(credit_card_payments
        .iter()
        .any(|payment| payment.amount == 250 && payment.credit_card_payment_type == "refund"));
}
