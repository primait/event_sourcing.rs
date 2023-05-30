use std::sync::{Arc, Mutex};

use esrs::{EventHandler, StoreEvent};

use crate::aggregate::{TestAggregate, TestEvent};

#[derive(Clone)]
pub struct TestEventHandler {
    pub total: Arc<Mutex<i32>>,
}

#[async_trait::async_trait]
impl EventHandler<TestAggregate> for TestEventHandler {
    async fn handle(&self, event: &StoreEvent<TestEvent>) {
        let mut guard = self.total.lock().unwrap();
        *guard = *guard + event.payload.add;
    }
}
