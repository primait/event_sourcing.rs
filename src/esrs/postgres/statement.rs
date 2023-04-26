use crate::Aggregate;

#[macro_export]
macro_rules! statement {
    ($file:expr, $ty:ty $(,)?) => {{
        format!(include_str!($file), format!("{}_events", <$ty>::NAME))
    }};
}

#[derive(Clone, Debug)]
pub struct Statements {
    create_table: String,
    create_index: String,
    create_unique_constraint: String,
    select_by_aggregate_id: String,
    select_all: String,
    insert: String,
    delete_by_aggregate_id: String,
}

impl Statements {
    pub fn new<A: Aggregate>() -> Self {
        Self {
            create_table: statement!("statements/create_table.sql", A),
            create_index: statement!("statements/create_index.sql", A),
            create_unique_constraint: statement!("statements/create_unique_constraint.sql", A),
            select_by_aggregate_id: statement!("statements/select_by_aggregate_id.sql", A),
            select_all: statement!("statements/select_all.sql", A),
            insert: statement!("statements/insert.sql", A),
            delete_by_aggregate_id: statement!("statements/delete_by_aggregate_id.sql", A),
        }
    }

    pub fn create_table(&self) -> &str {
        &self.create_table
    }

    pub fn create_index(&self) -> &str {
        &self.create_index
    }

    pub fn create_unique_constraint(&self) -> &str {
        &self.create_unique_constraint
    }

    pub fn by_aggregate_id(&self) -> &str {
        &self.select_by_aggregate_id
    }

    pub fn select_all(&self) -> &str {
        &self.select_all
    }

    pub fn insert(&self) -> &str {
        &self.insert
    }

    pub fn delete_by_aggregate_id(&self) -> &str {
        &self.delete_by_aggregate_id
    }
}
