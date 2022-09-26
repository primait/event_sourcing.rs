# Aggregate Merging

This example demonstrates a `Projector` which consumes events from two different `Aggregates`, and projects those events
to a single projection (in this case, a count of how many of each command the aggregates have received).
