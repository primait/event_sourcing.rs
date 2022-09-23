use async_trait::async_trait;

use crate::{AggregateManager, StoreEvent};

/// This trait is used to implement a `Policy`. A policy is intended to be an entity where to put
/// non-transactional side effects.
#[async_trait]
pub trait Policy<Manager>
where
    Self: PolicyClone<Manager>,
    Manager: AggregateManager,
{
    /// This function intercepts the event and, matching on the type of such event
    /// produces the appropriate side effects.
    /// The result is meant to catch generic errors.
    async fn handle_event(&self, event: &StoreEvent<Manager::Event>) -> Result<(), Manager::Error>;
}

/// This trait ensures that every `Box<dyn Policy<T>>` exposes a `clone_box` function, used later in
/// the other traits.
///
/// In the [`Policy`]  this bound is useful in order to let it be cloneable. Being cloneable is
/// mandatory in order to let the implementation of an `EventStore` be cloneable.
///
/// Keep in mind that while implementing `Policy<T>` for a struct it's needed for that struct to be
/// cloneable.
pub trait PolicyClone<Manager>
where
    Manager: AggregateManager,
{
    fn clone_box(&self) -> Box<dyn Policy<Manager> + Send + Sync>;
}

/// This trait create a default implementation of `PolicyClone` for every `T` where `T` is a `dyn Policy`,
/// to avoid the implementor of a [`Policy`] have to implement this trait and having this functionality
/// by default.
impl<T, Manager> PolicyClone<Manager> for T
where
    T: 'static + Policy<Manager> + Clone + Send + Sync,
    Manager: AggregateManager,
{
    fn clone_box(&self) -> Box<dyn Policy<Manager> + Send + Sync> {
        Box::new(self.clone())
    }
}

/// This trait implements [`Clone`] for every `Box<dyn Policy<T>>` calling [`PolicyClone`] `clone_box`
/// function.
impl<Manager> Clone for Box<dyn Policy<Manager> + Send + Sync>
where
    Manager: AggregateManager,
{
    fn clone(&self) -> Box<dyn Policy<Manager> + Send + Sync> {
        self.clone_box()
    }
}
