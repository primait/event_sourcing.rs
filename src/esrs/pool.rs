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

impl Pool<sqlx::Postgres> {
    pub async fn test_pool(url: &str) -> Result<Self, sqlx::Error> {
        let pool: sqlx::Pool<sqlx::Postgres> = PoolOptions::new().max_connections(1).connect(url).await?;
        sqlx::query("BEGIN").execute(&pool).await.map(|_| ())?;
        Ok(Self { pool, test: true })
    }
}

impl Pool<sqlx::Sqlite> {
    pub async fn test_pool(url: &str) -> Result<Self, sqlx::Error> {
        let pool: sqlx::Pool<sqlx::Sqlite> = PoolOptions::new().max_connections(1).connect(url).await?;
        sqlx::query("BEGIN").execute(&pool).await.map(|_| ())?;
        Ok(Self { pool, test: true })
    }
}

impl<T: Database> std::ops::Deref for Pool<T> {
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

impl<'a, T: Database> std::ops::Deref for Transaction<'a, T> {
    type Target = sqlx::Transaction<'a, T>;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.transaction
    }
}

impl<'c, DB> std::ops::DerefMut for Transaction<'c, DB>
where
    DB: Database,
{
    #[inline]
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.transaction
    }
}
