use std::ops::{Deref, DerefMut};

use sqlx::pool::PoolOptions;
use sqlx::Database;

pub struct Pool<T: Database> {
    pool: sqlx::Pool<T>,
    test: bool,
}

impl<T: Database> Pool<T> {
    pub async fn new(pool: sqlx::Pool<T>) -> Self {
        Self { pool, test: false }
    }

    pub async fn from_url(url: &str) -> Result<Self, sqlx::Error> {
        Ok(Self {
            pool: PoolOptions::<T>::new().connect(url).await?,
            test: false,
        })
    }

    pub async fn begin(&self) -> Result<Transaction<'_, T>, sqlx::Error> {
        Ok(Transaction {
            transaction: self.pool.begin().await?,
            test: self.test,
        })
    }

    pub async fn close(&self) {
        self.pool.close().await
    }
}

#[cfg(feature = "postgres")]
impl Pool<sqlx::Postgres> {
    pub async fn pg_test_pool(url: &str) -> Result<Self, sqlx::Error> {
        let pool: sqlx::Pool<sqlx::Postgres> = PoolOptions::new().max_connections(1).connect(url).await?;
        sqlx::query("BEGIN").execute(&pool).await.map(|_| ())?;
        Ok(Self { pool, test: true })
    }
}

#[cfg(feature = "sqlite")]
impl Pool<sqlx::Sqlite> {
    pub async fn sqlite_test_pool(url: &str) -> Result<Self, sqlx::Error> {
        let pool: sqlx::Pool<sqlx::Sqlite> = PoolOptions::new().max_connections(1).connect(url).await?;
        sqlx::query("BEGIN").execute(&pool).await.map(|_| ())?;
        Ok(Self { pool, test: true })
    }
}

impl<T: Database> Deref for Pool<T> {
    type Target = sqlx::Pool<T>;
    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl<T: Database> Clone for Pool<T> {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            test: self.test,
        }
    }
}

pub struct Transaction<'a, T: Database> {
    transaction: sqlx::Transaction<'a, T>,
    test: bool,
}

impl<'a, T: Database> Transaction<'a, T> {
    pub async fn commit(self) -> Result<(), sqlx::Error> {
        if self.test {
            Ok(())
        } else {
            self.transaction.commit().await
        }
    }

    pub async fn rollback(self) -> Result<(), sqlx::Error> {
        Ok(self.transaction.rollback().await?)
    }
}

impl<'a, T: Database> Deref for Transaction<'a, T> {
    type Target = T::Connection;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &*self.transaction
    }
}

impl<'c, DB> DerefMut for Transaction<'c, DB>
where
    DB: Database,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut *self.transaction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sqlx::postgres::PgRow;
    use sqlx::Postgres;

    #[tokio::test]
    async fn test_pool_test() {
        let database_url = std::env::var("DATABASE_URL").unwrap();
        let transaction_pool: Pool<Postgres> = Pool::pg_test_pool(database_url.as_str()).await.unwrap();
        let another_pool: Pool<Postgres> = Pool::from_url(database_url.as_str()).await.unwrap();

        sqlx::query("CREATE TABLE IF NOT EXISTS users (name TEXT NOT NULL)")
            .execute(&*another_pool)
            .await
            .unwrap();

        sqlx::query("INSERT INTO users (name) VALUES ($1)")
            .bind("Pippo")
            .execute(&*transaction_pool)
            .await
            .unwrap();

        let mut transaction = transaction_pool.begin().await.unwrap();

        sqlx::query("INSERT INTO users (name) VALUES ($1)")
            .bind("Gino")
            .execute(&mut *transaction)
            .await
            .unwrap();

        // Commit nothing
        transaction.commit().await.unwrap();

        let rows: Vec<PgRow> = sqlx::query("SELECT * FROM users")
            .fetch_all(&*another_pool)
            .await
            .unwrap();

        assert_eq!(rows.len(), 0);

        let _ = sqlx::query("DROP TABLE users").execute(&*another_pool).await.unwrap();
    }
}
