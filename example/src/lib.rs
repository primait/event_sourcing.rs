pub mod payment;

#[cfg(test)]
mod tests {
    use esrs::aggregate::Aggregate;
    use esrs::state::AggregateState;
    use esrs::store::postgres::PostgreStore;
    use esrs::{Identifiable, StoreParams};

    use crate::payment::async_impl::PaymentAggregate;
    use crate::payment::command::PaymentCommand;
    use crate::payment::error::Error;
    use crate::payment::event::PaymentEvent;
    use crate::payment::state::PaymentState;

    #[actix_rt::test]
    async fn check_amounts() {
        // Database connection params
        let connection_params: StoreParams = StoreParams {
            host: "localhost",
            port: None,
            user: "example",
            pass: "example",
            schema: "example",
        };

        // Payment aggregate store
        let payment_store: PostgreStore<PaymentEvent, Error> =
            PostgreStore::new_from_params(connection_params, PaymentAggregate::name(), vec![])
                .await
                .unwrap();

        // Payment aggregate
        let payment_aggregate: PaymentAggregate = PaymentAggregate::new(payment_store);

        let mut state: AggregateState<PaymentState> = AggregateState::default();
        assert_eq!(state.inner().total_amount, 0.0);

        // Cannot pay negative amount
        let result = payment_aggregate
            .handle_command(&mut state, PaymentCommand::Pay { amount: -10.0 })
            .await;

        assert_eq!(result.err().unwrap().to_string(), "Negative amount");

        // Cannot refund negative amount
        let result = payment_aggregate
            .handle_command(&mut state, PaymentCommand::Refund { amount: -10.0 })
            .await;

        assert_eq!(result.err().unwrap().to_string(), "Negative amount");

        // Cannot refund until total amount become negative (0 - 10)
        let result = payment_aggregate
            .handle_command(&mut state, PaymentCommand::Refund { amount: 10.0 })
            .await;

        assert_eq!(result.err().unwrap().to_string(), "Negative total amount");

        // Payment of 10 euros
        let mut state: AggregateState<PaymentState> = payment_aggregate
            .handle_command(&mut state, PaymentCommand::Pay { amount: 10.0 })
            .await
            .unwrap();

        assert_eq!(state.inner().total_amount, 10.0);

        // Payment of 10 euros. Total amount is 20
        let mut state: AggregateState<PaymentState> = payment_aggregate
            .handle_command(&mut state, PaymentCommand::Pay { amount: 10.0 })
            .await
            .unwrap();

        assert_eq!(state.inner().total_amount, 20.0);

        // Payment of 10 euros. Total amount is 30
        let mut state: AggregateState<PaymentState> = payment_aggregate
            .handle_command(&mut state, PaymentCommand::Pay { amount: 10.0 })
            .await
            .unwrap();

        assert_eq!(state.inner().total_amount, 30.0);

        assert_eq!(
            state.inner().total_amount,
            payment_aggregate.load(state.id()).await.unwrap().inner().total_amount
        );

        // Refund of 10 euros. Total amount is 20
        let state: AggregateState<PaymentState> = payment_aggregate
            .handle_command(&mut state, PaymentCommand::Refund { amount: 10.0 })
            .await
            .unwrap();

        assert_eq!(state.inner().total_amount, 20.0);
        assert_eq!(
            state.inner().total_amount,
            payment_aggregate.load(state.id()).await.unwrap().inner().total_amount
        );
    }
}
