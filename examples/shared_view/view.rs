use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::TABLE_NAME;

#[derive(sqlx::FromRow, Debug)]
pub struct SharedView {
    pub shared_id: Uuid,
    pub aggregate_id_a: Option<Uuid>,
    pub aggregate_id_b: Option<Uuid>,
    pub sum: i32,
}

pub enum Upsert {
    A {
        shared_id: Uuid,
        aggregate_id: Uuid,
        value: i32,
    },
    B {
        shared_id: Uuid,
        aggregate_id: Uuid,
        value: i32,
    },
}

impl SharedView {
    pub async fn by_id(
        shared_id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Option<Self>, sqlx::Error> {
        let query: String = format!("SELECT * FROM {} WHERE shared_id = $1", TABLE_NAME);

        sqlx::query_as::<_, Self>(query.as_str())
            .bind(shared_id)
            .fetch_optional(executor)
            .await
    }

    pub async fn upsert(ups: Upsert, executor: impl Executor<'_, Database = Postgres>) -> Result<(), sqlx::Error> {
        match ups {
            Upsert::A {
                shared_id,
                aggregate_id,
                value,
            } => upsert(shared_id, aggregate_id, value, "aggregate_id_a", executor).await,
            Upsert::B {
                shared_id,
                aggregate_id,
                value,
            } => upsert(shared_id, aggregate_id, value, "aggregate_id_b", executor).await,
        }
    }
}

async fn upsert(
    shared_id: Uuid,
    aggregate_id: Uuid,
    value: i32,
    id_field: &str,
    executor: impl Executor<'_, Database = Postgres>,
) -> Result<(), sqlx::Error> {
    let query = format!(
        "INSERT INTO {0} (shared_id, {1}, sum) VALUES ($1, $2, $3) \
        ON CONFLICT (shared_id) DO UPDATE SET {1} = $2, sum = {0}.sum + $3;",
        TABLE_NAME, id_field
    );

    sqlx::query(query.as_str())
        .bind(shared_id)
        .bind(aggregate_id)
        .bind(value)
        .fetch_optional(executor)
        .await
        .map(|_| ())
}
