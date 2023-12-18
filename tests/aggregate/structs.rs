use serde::{Deserialize, Serialize};

pub enum TestCommand {
    Single,
    Multi,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TestEvent {
    pub add: i32,
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for TestEvent {}

#[derive(Debug, thiserror::Error)]
pub enum TestError {}
