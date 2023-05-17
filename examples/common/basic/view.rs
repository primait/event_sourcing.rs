use sqlx::{Executor, Postgres};
use uuid::Uuid;

use crate::common::basic::BASIC_TABLE_NAME;

#[derive(sqlx::FromRow, Debug)]
pub struct BasicView {
    pub id: Uuid,
    pub content: String,
}

impl BasicView {
    pub async fn by_id(
        id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Option<Self>, sqlx::Error> {
        let query: String = format!("SELECT * FROM {} WHERE id = $1", BASIC_TABLE_NAME);

        sqlx::query_as::<_, Self>(query.as_str())
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    pub async fn upsert(
        id: Uuid,
        content: String,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        let query = format!(
            "INSERT INTO {0} (id, content) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET content = $2;",
            BASIC_TABLE_NAME
        );

        sqlx::query(query.as_str())
            .bind(id)
            .bind(content)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }
}
