use async_trait::async_trait;
use sqlx::postgres::PgQueryResult;
use sqlx::{Database, Error, Pool, Postgres, Transaction};

use crate::{statement, Aggregate};

#[async_trait]
pub trait MigrationsHandler<D>
where
    D: Database,
{
    async fn run<A>(pool: &Pool<D>) -> Result<(), Error>
    where
        A: Aggregate;
}

pub struct Migrations;

#[async_trait]
impl MigrationsHandler<Postgres> for Migrations {
    async fn run<A>(pool: &Pool<Postgres>) -> Result<(), Error>
    where
        A: Aggregate,
    {
        let mut transaction: Transaction<Postgres> = pool.begin().await?;

        let migrations: Vec<String> = vec![
            statement!("postgres/migrations/01_create_table.sql", A),
            statement!("postgres/migrations/02_create_index.sql", A),
            statement!("postgres/migrations/03_create_unique_constraint.sql", A),
        ];

        for migration in migrations {
            let _: PgQueryResult = sqlx::query(migration.as_str()).execute(&mut *transaction).await?;
        }

        transaction.commit().await
    }
}

#[cfg(test)]
mod tests {
    use sqlx::{Pool, Postgres};

    use crate::esrs::sql::migrations::{Migrations, MigrationsHandler};
    use crate::Aggregate;

    #[sqlx::test]
    async fn can_read_postgres_migrations(pool: Pool<Postgres>) {
        let result = Migrations::run::<TestAggregate>(&pool).await;
        assert!(result.is_ok());
    }

    pub struct TestAggregate;

    impl Aggregate for TestAggregate {
        const NAME: &'static str = "test";
        type State = ();
        type Command = ();
        type Event = ();
        type Error = ();

        fn handle_command(_state: &Self::State, _command: Self::Command) -> Result<Vec<Self::Event>, Self::Error> {
            Ok(vec![])
        }

        fn apply_event(_state: Self::State, _payload: Self::Event) -> Self::State {
            ()
        }
    }
}