use uuid::Uuid;

use crate::store::EventStoreLockGuard;
use crate::AggregateState;

/// The outcome of [`crate::manager::AggregateManager::lock_and_load`], akin to `Option<AggregateState<_>>`.
///
/// it contains the loaded [`AggregateState`] if found,
/// and ensures that the lock is preserved in all cases.
///
/// Releases the lock on drop, unless it gets transferred through its API.
pub struct LockedLoad<T>(LockedLoadInner<T>);

impl<T> LockedLoad<T> {
    /// Constructs a new instance from an AggregateState.
    /// It should contain its own lock.
    pub fn some(aggregate_state: AggregateState<T>) -> Self {
        Self(LockedLoadInner::Some(aggregate_state))
    }

    /// Constructs a new instance to hold the lock for an empty state.
    pub fn none(id: Uuid, lock: EventStoreLockGuard) -> Self {
        Self(LockedLoadInner::None { id, lock })
    }

    /// Checks if the AggregateState was found.
    pub fn is_some(&self) -> bool {
        matches!(self, Self(LockedLoadInner::Some(_)))
    }

    /// Extracts the contained AggregateState, or panics otherwise.
    pub fn unwrap(self) -> AggregateState<T> {
        let Self(inner) = self;
        match inner {
            LockedLoadInner::None { .. } => None,
            LockedLoadInner::Some(aggregate_state) => Some(aggregate_state),
        }
        .unwrap()
    }
}

impl<T> LockedLoad<T>
where
    T: Default,
{
    /// Extracts the contained AggregateState, otherwise returns a new one
    /// with the default internal state together with the lock.
    pub fn unwrap_or_default(self) -> AggregateState<T> {
        let Self(inner) = self;
        match inner {
            LockedLoadInner::None { id, lock } => {
                let mut aggregate_state = AggregateState::with_id(id);
                aggregate_state.set_lock(lock);
                aggregate_state
            }
            LockedLoadInner::Some(aggregate_state) => aggregate_state,
        }
    }
}

/// Encapsulates the logic to avoid exposing the internal behaviour.
/// It's essentially `Option<AggregateState<T>>`, but it also needs to
/// retain the lock information.
enum LockedLoadInner<T> {
    None { id: Uuid, lock: EventStoreLockGuard },
    Some(AggregateState<T>),
}
