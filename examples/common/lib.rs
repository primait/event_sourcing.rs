use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};
use thiserror::Error;

pub use a::*;
pub use b::*;
#[cfg(feature = "postgres")]
pub use basic::*;

mod a;
mod b;
#[cfg(feature = "postgres")]
mod basic;

pub async fn new_pool() -> Pool<Postgres> {
    let database_url: String = std::env::var("DATABASE_URL").unwrap();

    PgPoolOptions::new()
        .connect(database_url.as_str())
        .await
        .expect("Failed to create postgres pool")
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}
