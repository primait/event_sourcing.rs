use sqlx::{Database, Postgres};

use crate::Aggregate;

pub trait StatementsHandler<D>
where
    D: Database,
{
    fn new<A>() -> Self
    where
        A: Aggregate;
    fn table_name(&self) -> &str;
    fn by_aggregate_id(&self) -> &str;
    fn select_all(&self) -> &str;
    fn insert(&self) -> &str;
    fn delete_by_aggregate_id(&self) -> &str;
}

#[derive(Clone, Debug)]
pub struct Statements {
    table_name: String,
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
        let table_name: String = format!("{}_events", A::NAME);

        Self {
            table_name: "".to_string(),
            select_by_aggregate_id: format!(
                include_str!("postgres/statements/select_by_aggregate_id.sql"),
                table_name
            ),
            select_all: format!(include_str!("postgres/statements/select_all.sql"), table_name),
            insert: format!(include_str!("postgres/statements/insert.sql"), table_name),
            delete_by_aggregate_id: format!(
                include_str!("postgres/statements/delete_by_aggregate_id.sql"),
                table_name
            ),
        }
    }

    fn table_name(&self) -> &str {
        &self.table_name
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
