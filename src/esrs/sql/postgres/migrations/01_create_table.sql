CREATE TABLE IF NOT EXISTS {0}
(
    id uuid NOT NULL,
    aggregate_id uuid NOT NULL,
    payload jsonb NOT NULL,
    occurred_on TIMESTAMPTZ NOT NULL DEFAULT current_timestamp,
    sequence_number INT NOT NULL DEFAULT 1,
    CONSTRAINT {0}_pkey PRIMARY KEY (id)
)