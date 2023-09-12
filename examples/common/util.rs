use rand::prelude::IteratorRandom;
use sqlx::postgres::PgPoolOptions;
use sqlx::{Pool, Postgres};

#[allow(dead_code)]
pub async fn new_pool() -> Pool<Postgres> {
    let database_url: String = std::env::var("DATABASE_URL").unwrap();

    PgPoolOptions::new().connect(database_url.as_str()).await.unwrap()
}

#[allow(dead_code)]
pub fn random_letters() -> String {
    let mut rng = rand::thread_rng();
    let chars: String = (0..6)
        .map(|_| (b'a'..=b'z').choose(&mut rng).unwrap() as char)
        .collect();
    chars
}
