use rand::prelude::IteratorRandom;
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

    PgPoolOptions::new().connect(database_url.as_str()).await.unwrap()
}

#[derive(Debug, Error)]
pub enum CommonError {
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

pub fn random_letters() -> String {
    let mut rng = rand::thread_rng();
    let chars: String = (0..6)
        .into_iter()
        .map(|_| (b'a'..b'z').choose(&mut rng).unwrap() as char)
        .collect();
    chars
}
