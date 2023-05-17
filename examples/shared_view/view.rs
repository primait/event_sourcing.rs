use sqlx::{Executor, Postgres};
use uuid::Uuid;

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
        sqlx::query_as::<_, Self>("SELECT * FROM shared_view WHERE shared_id = $1")
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
        "INSERT INTO shared_view (shared_id, {0}, sum) VALUES ($1, $2, $3) \
        ON CONFLICT (shared_id) DO UPDATE SET {0} = $2, sum = shared_view.sum + $3;",
        id_field
    );

    sqlx::query(query.as_str())
        .bind(shared_id)
        .bind(aggregate_id)
        .bind(value)
        .fetch_optional(executor)
        .await
        .map(|_| ())
}
