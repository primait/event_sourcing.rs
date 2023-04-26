use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::pool::PoolConnection;
use sqlx::{Pool, Postgres};

use esrs::postgres::PgStore;
use esrs::types::SequenceNumber;
use esrs::{Aggregate, AggregateManager};
use esrs::{AggregateState, StoreEvent};

use crate::projector::CounterTransactionalEventHandler;
use crate::structs::{CounterCommand, CounterError, CounterEvent};

pub struct CounterAggregate {
    event_store: PgStore<Self>,
}

impl CounterAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        let event_store: PgStore<CounterAggregate> = PgStore::new(pool.clone())
            .set_transactional_queries(vec![Box::new(CounterTransactionalEventHandler)])
            .setup()
            .await?;

        Ok(Self { event_store })
    }
}

impl Aggregate for CounterAggregate {
    type State = ();
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn handle_command(_state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
        match command {
            Self::Command::Increment => Ok(vec![Self::Event::Incremented]),
            Self::Command::Decrement => Ok(vec![Self::Event::Decremented]),
        }
    }

    fn apply_event(state: Self::State, _: Self::Event) -> Self::State {
        // Take no action as this aggregate has no in memory state - only the projection
        state
    }
}

#[async_trait]
impl AggregateManager for CounterAggregate {
    type EventStore = PgStore<Self>;

    fn name() -> &'static str {
        "counter"
    }

    fn event_store(&self) -> &Self::EventStore {
        &self.event_store
    }

    async fn store_events(
        &self,
        aggregate_state: &mut AggregateState<Self::State>,
        events: Vec<Self::Event>,
    ) -> Result<Vec<StoreEvent<Self::Event>>, Self::Error> {
        // Here is the persistence flow customization.
        self.event_store
            .persist(|pool| async move {
                let mut connection: PoolConnection<Postgres> = pool.acquire().await?;
                let occurred_on: DateTime<Utc> = Utc::now();
                let mut store_events: Vec<StoreEvent<Self::Event>> = vec![];
                let starting_sequence_number: SequenceNumber = aggregate_state.next_sequence_number();

                for (index, event) in events.into_iter().enumerate() {
                    store_events.push(
                        self.event_store
                            .save_event(
                                *aggregate_state.id(),
                                event,
                                occurred_on,
                                starting_sequence_number + index as i32,
                                &mut *connection,
                            )
                            .await?,
                    )
                }

                // Acquiring the list of projectors early, as it is an expensive operation.
                let transactional_queries = self.event_store().transactional_queries();
                for store_event in store_events.iter() {
                    for transactional_query in transactional_queries.iter() {
                        transactional_query.handle(store_event, &mut connection).await?;
                    }
                }

                // We need to drop the lock on the aggregate state here as:
                // 1. the events have already been persisted, hence the DB has the latest aggregate;
                // 2. the policies below might need to access this aggregate atomically (causing a deadlock!).
                drop(aggregate_state.take_lock());

                // Acquiring the list of policies early, as it is an expensive operation.
                let queries = self.event_store().queries();
                for store_event in store_events.iter() {
                    for query in queries.iter() {
                        // We want to just log errors instead of return them. This is the customization
                        // we wanted.
                        query.handle(store_event).await;
                    }
                }

                Ok(store_events)
            })
            .await
    }
}
