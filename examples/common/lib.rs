use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

pub use a::*;
pub use b::*;

mod a;
mod b;

pub async fn new_pool() -> Pool<Postgres> {
    let database_url: String = std::env::var("DATABASE_URL").unwrap();

    PgPoolOptions::new()
        .connect(database_url.as_str())
        .await
        .expect("Failed to create postgres pool")
}
