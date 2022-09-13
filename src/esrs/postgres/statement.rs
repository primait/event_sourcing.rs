pub struct Statements {
    create_table: String,
    create_index: String,
    create_unique_constraint: String,
    select: String,
    select_all: String,
    insert: String,
    delete: String,
}

impl Statements {
    pub fn new(name: &str) -> Self {
        let name: String = format!("{}_events", name);

        Self {
            create_table: create_table_statement(name.as_str()),
            create_index: create_index_statement(name.as_str()),
            create_unique_constraint: create_unique_constraint_statement(name.as_str()),
            select: select_statement(name.as_str()),
            select_all: select_all_statement(name.as_str()),
            insert: insert_statement(name.as_str()),
            delete: delete_statement(name.as_str()),
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
        &self.select
    }

    pub fn select_all(&self) -> &str {
        &self.select_all
    }

    pub fn insert(&self) -> &str {
        &self.insert
    }

    pub fn delete_by_aggregate_id(&self) -> &str {
        &self.delete
    }
}

fn create_table_statement(table_name: &str) -> String {
    format!(
        "CREATE TABLE IF NOT EXISTS {0}
        (
          id uuid NOT NULL,
          aggregate_id uuid NOT NULL,
          payload jsonb NOT NULL,
          occurred_on TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
          sequence_number INT NOT NULL DEFAULT 1,
          CONSTRAINT {0}_pkey PRIMARY KEY (id)
        )",
        table_name
    )
}

fn create_index_statement(table_name: &str) -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {0}_aggregate_id ON {0} USING btree (((payload ->> 'id'::text)))",
        table_name
    )
}

fn create_unique_constraint_statement(table_name: &str) -> String {
    format!(
        "CREATE UNIQUE INDEX IF NOT EXISTS {0}_aggregate_id_sequence_number ON {0}(aggregate_id, sequence_number)",
        table_name
    )
}

fn select_all_statement(table_name: &str) -> String {
    format!("SELECT * FROM {} ORDER BY sequence_number ASC", table_name)
}

fn select_statement(table_name: &str) -> String {
    format!(
        "SELECT * FROM {} WHERE aggregate_id = $1 ORDER BY sequence_number ASC",
        table_name
    )
}

fn insert_statement(table_name: &str) -> String {
    format!(
        "INSERT INTO {} (id, aggregate_id, payload, occurred_on, sequence_number) VALUES ($1, $2, $3, $4, $5)",
        table_name
    )
}

fn delete_statement(table_name: &str) -> String {
    format!("DELETE FROM {} WHERE aggregate_id = $1", table_name)
}
