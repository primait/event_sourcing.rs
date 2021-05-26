pub struct Queries {
    select: String,
    select_all: String,
    insert: String,
    delete: String,
}

impl Queries {
    pub fn new(name: &str) -> Self {
        Self {
            select: select_statement(name),
            select_all: select_all_statement(name),
            insert: insert_statement(name),
            delete: delete_statement(name),
        }
    }

    pub fn select(&self) -> &str {
        &self.select
    }
    pub fn select_all(&self) -> &str {
        &self.select_all
    }
    pub fn insert(&self) -> &str {
        &self.insert
    }
    pub fn delete(&self) -> &str {
        &self.delete
    }
}

pub fn create_table_statement(aggregate_name: &str) -> String {
    format!(
        "
    CREATE TABLE IF NOT EXISTS {0}_events
    (
      id uuid NOT NULL,
      aggregate_id uuid NOT NULL,
      payload jsonb NOT NULL,
      occurred_on TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
      sequence_number INT NOT NULL DEFAULT 1,
      CONSTRAINT {0}_events_pkey PRIMARY KEY (id)
    )
    ",
        aggregate_name
    )
}

fn select_all_statement(aggregate_name: &str) -> String {
    format!("SELECT * FROM {}_events", aggregate_name)
}

fn select_statement(aggregate_name: &str) -> String {
    format!("SELECT * FROM {}_events WHERE aggregate_id = $1", aggregate_name)
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
