use sqlx::{Executor, Pool, Postgres};
use uuid::Uuid;

use crate::common::util::random_letters;

#[derive(sqlx::FromRow, Debug)]
pub struct BasicViewRow {
    pub id: Uuid,
    pub content: String,
}

#[derive(Clone)]
pub struct BasicView {
    table_name: String,
}

#[allow(dead_code)]
impl BasicView {
    pub async fn new(table_name: &str, pool: &Pool<Postgres>) -> Self {
        let table_name: String = format!("{}_{}", random_letters(), table_name);

        let query: String = format!(
            "CREATE TABLE IF NOT EXISTS {} (id uuid PRIMARY KEY NOT NULL, content VARCHAR)",
            table_name
        );

        let _ = sqlx::query(query.as_str()).execute(pool).await.unwrap();

        Self { table_name }
    }

    pub async fn by_id(
        &self,
        id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Option<BasicViewRow>, sqlx::Error> {
        let query: String = format!("SELECT * FROM {} WHERE id = $1", &self.table_name);

        sqlx::query_as::<_, BasicViewRow>(query.as_str())
            .bind(id)
            .fetch_optional(executor)
            .await
    }

    pub async fn upsert(
        &self,
        id: Uuid,
        content: String,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<(), sqlx::Error> {
        let query = format!(
            "INSERT INTO {0} (id, content) VALUES ($1, $2) ON CONFLICT (id) DO UPDATE SET content = $2;",
            &self.table_name
        );

        sqlx::query(query.as_str())
            .bind(id)
            .bind(content)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub async fn delete(&self, id: Uuid, executor: impl Executor<'_, Database = Postgres>) -> Result<(), sqlx::Error> {
        let query = format!("DELETE FROM {0} WHERE id = $1;", &self.table_name);

        sqlx::query(query.as_str())
            .bind(id)
            .fetch_optional(executor)
            .await
            .map(|_| ())
    }

    pub fn table_name(&self) -> &str {
        &self.table_name
    }
}
