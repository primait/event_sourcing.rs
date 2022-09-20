use async_trait::async_trait;

use crate::{AggregateManager, StoreEvent};

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

pub trait PolicyClone<Manager>
where
    Manager: AggregateManager,
{
    fn clone_box(&self) -> Box<dyn Policy<Manager> + Send + Sync>;
}

impl<T, Manager> PolicyClone<Manager> for T
where
    T: 'static + Policy<Manager> + Clone + Send + Sync,
    Manager: AggregateManager,
{
    fn clone_box(&self) -> Box<dyn Policy<Manager> + Send + Sync> {
        Box::new(self.clone())
    }
}

impl<Manager> Clone for Box<dyn Policy<Manager> + Send + Sync>
where
    Manager: AggregateManager,
{
    fn clone(&self) -> Box<dyn Policy<Manager> + Send + Sync> {
        self.clone_box()
    }
}
