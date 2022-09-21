use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::pool::PoolConnection;
use sqlx::{Pool, Postgres};

use esrs::postgres::PgStore;
use esrs::types::SequenceNumber;
use esrs::StoreEvent;
use esrs::{Aggregate, AggregateManager, AggregateState};

use crate::projector::CounterProjector;
use crate::structs::{CounterCommand, CounterError, CounterEvent};

pub struct CounterAggregate {
    event_store: PgStore<Self>,
}

impl CounterAggregate {
    pub async fn new(pool: &Pool<Postgres>) -> Result<Self, CounterError> {
        let mut event_store: PgStore<CounterAggregate> = PgStore::new(pool.clone()).setup().await?;
        event_store.add_projector(Box::new(CounterProjector));
        Ok(Self { event_store })
    }
}

impl Aggregate for CounterAggregate {
    type State = ();
    type Command = CounterCommand;
    type Event = CounterEvent;
    type Error = CounterError;

    fn handle_command(
        _state: &AggregateState<Self::State>,
        command: Self::Command,
    ) -> Result<Vec<Self::Event>, Self::Error> {
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
        aggregate_state: &AggregateState<Self::State>,
        events: Vec<Self::Event>,
    ) -> Result<Vec<StoreEvent<Self::Event>>, Self::Error> {
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

                for store_event in store_events.iter() {
                    for projector in self.event_store.projectors() {
                        projector.project(store_event, &mut connection).await?;
                    }
                }

                for store_event in store_events.iter() {
                    for policy in self.event_store.policies() {
                        // We want to just log errors instead of return them
                        match policy.handle_event(store_event).await {
                            Ok(_) => (),
                            Err(error) => println!("{:?}", error),
                        }
                    }
                }

                Ok(store_events)
            })
            .await
    }
}
