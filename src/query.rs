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

pub fn create_id_index_statement(aggregate_name: &str) -> String {
    format!(
        "CREATE INDEX IF NOT EXISTS {0}_events_aggregate_id ON public.{0}_events USING btree (((payload ->> 'id'::text)))",
        aggregate_name
    )
}

pub fn create_aggregate_id_index_statement(aggregate_name: &str) -> String {
    format!(
        "CREATE UNIQUE INDEX IF NOT EXISTS {0}_events_aggregate_id_sequence_number ON {0}_events(aggregate_id, sequence_number)",
        aggregate_name
    )
}

pub fn select_statement(aggregate_name: &str) -> String {
    format!("SELECT * FROM {}_events WHERE aggregate_id = $1", aggregate_name)
}

pub fn insert_statement(aggregate_name: &str) -> String {
    format!(
        "
    INSERT INTO {}_events
    (id, aggregate_id, payload, occurred_on, sequence_number)
    VALUES ($1, $2, $3, $4, $5)
    RETURNING *
    ",
        aggregate_name
    )
}
