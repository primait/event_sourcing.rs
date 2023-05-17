use esrs::Aggregate;
use sqlx::{Pool, Postgres};

use crate::aggregate::TestAggregate;
use esrs::postgres::{PgStore, PgStoreBuilder};

#[sqlx::test]
async fn builder_can_skip_migrations_test(pool: Pool<Postgres>) {
    let store: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone())
        .without_running_migrations()
        .try_build()
        .await
        .unwrap();

    assert!(!table_exists(store.table_name(), &pool).await);
}

#[sqlx::test]
async fn builder_run_migrations_test(pool: Pool<Postgres>) {
    let table_name: String = format!("{}_events", TestAggregate::NAME);
    assert!(!table_exists(table_name.as_str(), &pool).await);

    let _: PgStore<TestAggregate> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    assert!(table_exists(table_name.as_str(), &pool).await);

    drop(table_name.as_str(), &pool).await;
}

async fn table_exists(table_name: &str, pool: &Pool<Postgres>) -> bool {
    !sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
        .bind(table_name)
        .fetch_all(pool)
        .await
        .unwrap()
        .is_empty()
}

async fn drop(table_name: &str, pool: &Pool<Postgres>) {
    let query: String = format!("DROP TABLE IF EXISTS {}", table_name);
    let _ = sqlx::query(query.as_str())
        .bind(table_name)
        .execute(pool)
        .await
        .unwrap();
}
