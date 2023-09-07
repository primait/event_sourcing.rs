use std::fmt::{Display, Formatter};

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
impl esrs::event::Upcaster for TestEvent {
    fn upcast(value: serde_json::Value, _version: Option<i32>) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

pub enum TestError {}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "test error")
    }
}
