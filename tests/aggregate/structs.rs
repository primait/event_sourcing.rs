use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

pub enum TestCommand {
    Single,
    Multi,
}

#[derive(Clone, Serialize, Deserialize, Debug)]
#[cfg_attr(feature = "upcasting", derive(esrs::Event))]
pub struct TestEvent {
    pub add: i32,
}

#[cfg(feature = "upcasting")]
impl esrs::event::Upcaster for TestEvent {
    fn upcast(value: serde_json::Value, _version: Option<i32>) -> Result<Self, serde_json::Error> {
        serde_json::from_value(value)
    }
}

#[derive(Debug)]
pub enum TestError {
    #[cfg(feature = "postgres")]
    Sqlx(sqlx::Error),
    Json(serde_json::Error),
}

impl Display for TestError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "test error")
    }
}

impl std::error::Error for TestError {}

#[cfg(feature = "postgres")]
impl From<sqlx::Error> for TestError {
    fn from(value: sqlx::Error) -> Self {
        TestError::Sqlx(value)
    }
}

impl From<serde_json::Error> for TestError {
    fn from(value: serde_json::Error) -> Self {
        TestError::Json(value)
    }
}
