use sqlx::postgres::PgQueryResult;
use sqlx::{Pool, Postgres};

use crate::esrs::postgres::index;
use crate::esrs::query;

pub async fn run_preconditions(pool: &Pool<Postgres>, aggregate_name: &str) -> Result<(), sqlx::Error> {
    // Create table if not exists
    let _: PgQueryResult = sqlx::query(query::create_table_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    // Create 2 indexes if not exist
    let _: PgQueryResult = sqlx::query(index::create_id_index_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    let _: PgQueryResult = sqlx::query(index::create_aggregate_id_index_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    Ok(())
}
