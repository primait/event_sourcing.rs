use sqlx::{Database, Postgres};

use crate::{statement, Aggregate};

pub trait StatementsHandler<D>
where
    D: Database,
{
    fn new<A>() -> Self
    where
        A: Aggregate;
    fn by_aggregate_id(&self) -> &str;
    fn select_all(&self) -> &str;
    fn insert(&self) -> &str;
    fn delete_by_aggregate_id(&self) -> &str;
}

#[derive(Clone, Debug)]
pub struct Statements {
    select_by_aggregate_id: String,
    select_all: String,
    insert: String,
    delete_by_aggregate_id: String,
}

impl StatementsHandler<Postgres> for Statements {
    fn new<A>() -> Self
    where
        A: Aggregate,
    {
        Self {
            select_by_aggregate_id: statement!("postgres/statements/select_by_aggregate_id.sql", A),
            select_all: statement!("postgres/statements/select_all.sql", A),
            insert: statement!("postgres/statements/insert.sql", A),
            delete_by_aggregate_id: statement!("postgres/statements/delete_by_aggregate_id.sql", A),
        }
    }

    fn by_aggregate_id(&self) -> &str {
        &self.select_by_aggregate_id
    }

    fn select_all(&self) -> &str {
        &self.select_all
    }

    fn insert(&self) -> &str {
        &self.insert
    }

    fn delete_by_aggregate_id(&self) -> &str {
        &self.delete_by_aggregate_id
    }
}
