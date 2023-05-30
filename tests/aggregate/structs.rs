use std::fmt::{Display, Formatter};

pub enum TestCommand {
    Single,
    Multi,
}

#[derive(Clone, serde::Serialize, serde::Deserialize, Debug)]
pub struct TestEvent {
    pub add: i32,
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
