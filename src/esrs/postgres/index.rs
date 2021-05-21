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
