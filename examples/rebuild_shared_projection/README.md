# Multiple aggregate rebuild

An example of rebuilding a projection which derives from two aggregates using a stream.

In this example the two streams read the full table. Before rebuilding the whole table a new transaction is created 
and the table gets truncated. If something fails within the transaction the table will keep the previous data.
