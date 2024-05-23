mod locked_load;

pub use locked_load::LockedLoad;

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
    /// The store transactionally persists the events - recording them in the aggregate instance's history.
    ///
    /// On success, the updated aggregate state is returned.
    ///
    /// Returns two layers of errors:
    /// - `Err(_)` if the aggregate handled the command but the outcome failed to be recorded;
    /// - `Ok(Err(_))` if the aggregate denied the command.
    pub async fn handle_command(
        &self,
        mut aggregate_state: AggregateState<<E::Aggregate as Aggregate>::State>,
        command: <E::Aggregate as Aggregate>::Command,
    ) -> Result<Result<<E::Aggregate as Aggregate>::State, <E::Aggregate as Aggregate>::Error>, E::Error> {
        match <E::Aggregate as Aggregate>::handle_command(aggregate_state.inner(), command) {
            Err(domain_error) => Ok(Err(domain_error)),
            Ok(events) => match self.event_store.persist(&mut aggregate_state, events).await {
                Ok(store_events) => Ok(Ok(aggregate_state
                    .apply_store_events(store_events, <E::Aggregate as Aggregate>::apply_event)
                    .into_inner())),
                Err(operational_error) => Err(operational_error),
            },
        }
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
    /// The returned [`LockedLoad`] contains the outcome of the load and is responsible for correctly managing the lock.
    pub async fn lock_and_load(
        &self,
        aggregate_id: impl Into<Uuid> + Send,
    ) -> Result<LockedLoad<<E::Aggregate as Aggregate>::State>, E::Error> {
        let id = aggregate_id.into();
        let guard = self.event_store.lock(id).await?;
        let aggregate_state = self.load(id).await?;

        Ok(match aggregate_state {
            Some(mut aggregate_state) => {
                aggregate_state.set_lock(guard);
                LockedLoad::some(aggregate_state)
            }
            None => LockedLoad::none(id, guard),
        })
    }

    /// `delete` should either complete the aggregate instance, along with all its associated events
    /// and transactional read side projections, or fail.
    pub async fn delete(&self, aggregate_id: impl Into<Uuid> + Send) -> Result<(), E::Error> {
        self.event_store.delete(aggregate_id.into()).await
    }
}
