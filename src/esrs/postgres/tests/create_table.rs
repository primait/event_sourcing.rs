use sqlx::{Pool, Postgres};

use crate::esrs::postgres::create_table;
use crate::esrs::postgres::tests::TestError;

#[sqlx::test]
fn create_table_test(pool: Pool<Postgres>) {
    let table_name: &str = "test_events";

    let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
        .bind(table_name)
        .fetch_all(&pool)
        .await
        .unwrap();

    assert!(rows.is_empty());

    create_table::<TestError>(table_name, &pool).await.unwrap();

    let rows = sqlx::query("SELECT table_name FROM information_schema.columns WHERE table_name = $1")
        .bind(table_name)
        .fetch_all(&pool)
        .await
        .unwrap();

    assert!(!rows.is_empty());

    let rows = sqlx::query("SELECT indexname FROM pg_indexes WHERE tablename = $1")
        .bind(table_name)
        .fetch_all(&pool)
        .await
        .unwrap();

    // primary key, aggregate_id, aggregate_id-sequence_number
    assert_eq!(rows.len(), 3);
}
