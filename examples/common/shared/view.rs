use sqlx::{Executor, Pool, Postgres};
use uuid::Uuid;

use crate::common::random_letters;

#[derive(Clone)]
pub struct SharedView {
    pub table_name: String,
}

#[derive(sqlx::FromRow, Debug)]
pub struct SharedViewRow {
    pub shared_id: Uuid,
    pub aggregate_id_a: Option<Uuid>,
    pub aggregate_id_b: Option<Uuid>,
    pub sum: i32,
}

pub enum UpsertSharedView {
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
    pub async fn new(table_name: &str, pool: &Pool<Postgres>) -> Self {
        let table_name: String = format!("{}_{}", random_letters(), table_name);

        let query: String = format!(
            "CREATE TABLE IF NOT EXISTS {} (shared_id uuid PRIMARY KEY NOT NULL, aggregate_id_a uuid, aggregate_id_b uuid, sum INTEGER)",
            table_name
        );

        let _ = sqlx::query(query.as_str()).execute(pool).await.unwrap();

        Self { table_name }
    }

    pub async fn by_id(
        &self,
        shared_id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Option<SharedViewRow>, sqlx::Error> {
        let query: String = format!("SELECT * FROM {} WHERE shared_id = $1", &self.table_name);

        sqlx::query_as::<_, SharedViewRow>(query.as_str())
            .bind(shared_id)
            .fetch_optional(executor)
            .await
    }

    pub async fn upsert(
        &self,
        ups: UpsertSharedView,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        match ups {
            UpsertSharedView::A {
                shared_id,
                aggregate_id,
                value,
            } => {
                upsert(
                    self.table_name(),
                    shared_id,
                    aggregate_id,
                    value,
                    "aggregate_id_a",
                    executor,
                )
                .await
            }
            UpsertSharedView::B {
                shared_id,
                aggregate_id,
                value,
            } => {
                upsert(
                    self.table_name(),
                    shared_id,
                    aggregate_id,
                    value,
                    "aggregate_id_b",
                    executor,
                )
                .await
            }
        }
    }

    pub async fn update_sum(
        &self,
        shared_id: Uuid,
        value: i32,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        let query = format!("UPDATE {} SET sum = $2 WHERE shared_id = $1", self.table_name());

        sqlx::query(query.as_str())
            .bind(shared_id)
            .bind(value)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}

async fn upsert(
    table_name: &str,
    shared_id: Uuid,
    aggregate_id: Uuid,
    value: i32,
    id_field: &str,
    executor: impl Executor<'_, Database = Postgres>,
) -> Result<(), sqlx::Error> {
    let query = format!(
        "INSERT INTO {0} (shared_id, {1}, sum) VALUES ($1, $2, $3) \
        ON CONFLICT (shared_id) DO UPDATE SET {1} = $2, sum = {0}.sum + $3;",
        table_name, id_field
    );

    sqlx::query(query.as_str())
        .bind(shared_id)
        .bind(aggregate_id)
        .bind(value)
        .fetch_optional(executor)
        .await
        .map(|_| ())
}
