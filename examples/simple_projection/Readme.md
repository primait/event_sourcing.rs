An example of a single `Aggregate`, with a single `Projector`. Commands to update a counter (increment or decrement) are
received by the `CounterAggregate`, transformed into a `CounterEvent` which is then processed by the `CounterProjector`,
which runs a SQL query against the "counters" table, updating a projected view of the events (the counter value)
