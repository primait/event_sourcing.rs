use sqlx::postgres::PgQueryResult;
use sqlx::{Pool, Postgres};

pub async fn run(pool: &Pool<Postgres>, aggregate_name: &str) -> Result<(), sqlx::Error> {
    // Create table if not exists
    let _: PgQueryResult = sqlx::query(create_table_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    // Create 2 indexes if not exist
    let _: PgQueryResult = sqlx::query(create_id_index_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    let _: PgQueryResult = sqlx::query(create_aggregate_id_index_statement(aggregate_name).as_str())
        .execute(pool)
        .await?;

    Ok(())
}

fn create_table_statement(aggregate_name: &str) -> String {
    format!(
        "
        CREATE TABLE IF NOT EXISTS {0}_events
        (
          id uuid NOT NULL,
          aggregate_id uuid NOT NULL,
          payload jsonb NOT NULL,
          occurred_on TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
          sequence_number INT NOT NULL DEFAULT 1,
          CONSTRAINT {0}_events_pkey PRIMARY KEY (id)
        )
        ",
        aggregate_name
    )
}

fn create_id_index_statement(aggregate_name: &str) -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {0}_events_aggregate_id ON {0}_events USING btree (((payload ->> 'id'::text)))",
        aggregate_name
    )
}

fn create_aggregate_id_index_statement(aggregate_name: &str) -> String {
    format!(
        "CREATE UNIQUE INDEX IF NOT EXISTS {0}_events_aggregate_id_sequence_number ON {0}_events(aggregate_id, sequence_number)",
        aggregate_name
    )
}
