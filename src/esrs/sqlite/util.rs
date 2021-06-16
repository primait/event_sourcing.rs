use sqlx::{Pool, Sqlite};

use crate::esrs::query;

pub async fn run_preconditions(pool: &Pool<Sqlite>, aggregate_name: &str) -> Result<(), sqlx::Error> {
    // Create table if not exists
    sqlx::query(query::create_table_statement(aggregate_name).as_str())
        .execute(pool)
        .await
        .map(|_| ())
}
