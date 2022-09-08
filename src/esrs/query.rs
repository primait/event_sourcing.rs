pub struct Queries {
    select: String,
    insert: String,
    delete: String,
}

impl Queries {
    pub fn new(name: &str) -> Self {
        Self {
            select: select_statement(name),
            insert: insert_statement(name),
            delete: delete_statement(name),
        }
    }

    pub fn select(&self) -> &str {
        &self.select
    }

    pub fn insert(&self) -> &str {
        &self.insert
    }

    pub fn delete(&self) -> &str {
        &self.delete
    }
}

fn select_statement(aggregate_name: &str) -> String {
    format!(
        "SELECT * FROM {}_events WHERE aggregate_id = $1 ORDER BY sequence_number ASC",
        aggregate_name
    )
}

fn insert_statement(aggregate_name: &str) -> String {
    format!(
        "
    INSERT INTO {}_events
    (id, aggregate_id, payload, occurred_on, sequence_number)
    VALUES ($1, $2, $3, $4, $5)
    ",
        aggregate_name
    )
}

fn delete_statement(aggregate_name: &str) -> String {
    format!("DELETE FROM {}_events WHERE aggregate_id = $1", aggregate_name)
}
