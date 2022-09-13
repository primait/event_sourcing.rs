mod create_table;
mod store;

#[derive(Debug)]
pub struct TestError;

impl From<sqlx::Error> for TestError {
    fn from(_: sqlx::Error) -> Self {
        TestError
    }
}

impl From<serde_json::Error> for TestError {
    fn from(_: serde_json::Error) -> Self {
        TestError
    }
}
