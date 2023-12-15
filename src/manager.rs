use uuid::Uuid;

use crate::store::{EventStore, StoreEvent};
use crate::{Aggregate, AggregateState};

/// The AggregateManager is responsible for coupling the Aggregate with a Store, so that the events
/// can be persisted when handled, and the state can be reconstructed by loading and apply events sequentially.
///
/// The basic APIs are:
/// 1. handle_command
/// 2. load
/// 3. lock_and_load
pub struct AggregateManager<E>
where
    E: EventStore,
{
    event_store: E,
}

impl<E> AggregateManager<E>
where
    E: EventStore,
{
    /// Creates a new instance of an [`AggregateManager`].
    pub fn new(event_store: E) -> Self {
        Self { event_store }
    }

    /// Validates and handles the command onto the given state, and then passes the events to the store.
    ///
    /// The store transactional persists the events - recording it in the aggregate instance's history.
    pub async fn handle_command<Er>(
        &self,
        mut aggregate_state: AggregateState<<E::Aggregate as Aggregate>::State>,
        command: <E::Aggregate as Aggregate>::Command,
    ) -> Result<(), Er>
    where
        Er: From<E::Error> + From<<E::Aggregate as Aggregate>::Error> + std::error::Error,
    {
        let events: Vec<<E::Aggregate as Aggregate>::Event> =
            <E::Aggregate as Aggregate>::handle_command(aggregate_state.inner(), command)?;

        self.event_store.persist(&mut aggregate_state, events).await?;

        Ok(())
    }

    /// Loads an aggregate instance from the event store, by applying previously persisted events onto
    /// the aggregate state by order of their sequence number.
    pub async fn load(
        &self,
        aggregate_id: impl Into<Uuid> + Send,
    ) -> Result<Option<AggregateState<<E::Aggregate as Aggregate>::State>>, E::Error> {
        let aggregate_id: Uuid = aggregate_id.into();

        let store_events: Vec<StoreEvent<<E::Aggregate as Aggregate>::Event>> = self
            .event_store
            .by_aggregate_id(aggregate_id)
            .await?
            .into_iter()
            .collect();

        Ok(if store_events.is_empty() {
            None
        } else {
            let aggregate_state = AggregateState::with_id(aggregate_id);
            Some(aggregate_state.apply_store_events(store_events, <E::Aggregate as Aggregate>::apply_event))
        })
    }

    /// Acquires a lock on this aggregate instance, and only then loads it from the event store,
    /// by applying previously persisted events onto the aggregate state by order of their sequence number.
    ///
    /// The lock is contained in the returned `AggregateState`, and released when this is dropped.
    /// It can also be extracted with the `take_lock` method for more advanced uses.
    pub async fn lock_and_load(
        &self,
        aggregate_id: impl Into<Uuid> + Send,
    ) -> Result<Option<AggregateState<<E::Aggregate as Aggregate>::State>>, E::Error> {
        let id = aggregate_id.into();
        let guard = self.event_store.lock(id).await?;

        Ok(self.load(id).await?.map(|mut state| {
            state.set_lock(guard);
            state
        }))
    }

    /// `delete` should either complete the aggregate instance, along with all its associated events
    /// and transactional read side projections, or fail.
    pub async fn delete(&self, aggregate_id: impl Into<Uuid> + Send) -> Result<(), E::Error> {
        self.event_store.delete(aggregate_id.into()).await
    }

    /// Returns the internal event store
    pub fn event_store(&self) -> &E {
        &self.event_store
    }
}
