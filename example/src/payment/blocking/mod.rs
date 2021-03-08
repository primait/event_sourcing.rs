// use esrs::aggregate::Aggregate;
// use esrs::state::AggregateState;
// use esrs::store::{EventStore, StoreEvent};
//
// use crate::payment::command::PaymentCommand;
// use crate::payment::error::Error;
// use crate::payment::event::PaymentEvent;
// use crate::payment::state::PaymentState;
// use super::store::VoidStore;
//
// const PAYMENT: &str = "payment";
//
// pub struct PaymentAggregate {
//     event_store: VoidStore,
// }
//
// impl PaymentAggregate {
//     pub fn new() -> Self {
//         Self { event_store: VoidStore }
//     }
// }
//
// impl Aggregate for PaymentAggregate {
//     type State = PaymentState;
//     type Command = PaymentCommand;
//     type Event = PaymentEvent;
//     type Error = Error;
//
//     fn name(&self) -> &'static str {
//         PAYMENT
//     }
//
//     fn event_store(&self) -> &dyn EventStore<Self::Event, Self::Error> {
//         &self.event_store
//     }
//
//     fn apply_event(
//         aggregate_state: AggregateState<PaymentState>,
//         event: &StoreEvent<Self::Event>,
//     ) -> AggregateState<PaymentState> {
//         match event.payload() {
//             PaymentEvent::Payed { amount } => AggregateState {
//                 state: aggregate_state.state.add_amount(*amount),
//                 ..aggregate_state
//             },
//             PaymentEvent::Refunded { amount } => AggregateState {
//                 state: aggregate_state.state.sub_amount(*amount),
//                 ..aggregate_state
//             },
//         }
//     }
//
//     fn validate_command(_state: &AggregateState<PaymentState>, cmd: &Self::Command) -> Result<(), Self::Error> {
//         match cmd {
//             PaymentCommand::Pay { amount } if *amount < 0.0 => Err(Self::Error::NegativeAmount),
//             PaymentCommand::Pay { .. } => Ok(()),
//             PaymentCommand::Refund { .. } => Ok(()),
//         }
//     }
//
//     fn do_handle_command(
//         &self,
//         state: &AggregateState<PaymentState>,
//         cmd: Self::Command,
//     ) -> Result<StoreEvent<Self::Event>, Self::Error> {
//         match cmd {
//             PaymentCommand::Pay { amount } => self.persist(state, PaymentEvent::Payed { amount }),
//             PaymentCommand::Refund { amount } => self.persist(state, PaymentEvent::Payed { amount }),
//         }
//     }
// }
