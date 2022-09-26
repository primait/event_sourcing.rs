CREATE TABLE counters (
    "counter_id" UUID PRIMARY KEY NOT NULL,
    "counter_a_id" UUID,
    "counter_b_id" UUID,
    "count_a" INTEGER NOT NULL,
    "count_b" INTEGER NOT NULL
);
