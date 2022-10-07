# Rebuild strategies

In this example there are some strategies you may need when there's the need to rebuild one or more shared projections.

## Rebuild single projection all at once 

In this example the user may want to delete the entire table and rebuild all its entries. This is done in a 
transaction truncating all the table content and then rebuild all the events retrieved in the moment the transaction 
is opened.

## Rebuild single projection per aggregate id

In this example the table is not locked in a transaction but the rebuild is made opening a transaction for every id 
fetched by a pre-made query. In that transaction every row having a matching aggregate id is deleted and then 
reprojected.

## Rebuild shared projection streaming

In this example the user need to rebuild two (or more) projections. This could be achieved by opening a transaction, 
getting all the events from both the stores and deleting both the projection tables. After that (streaming all the 
events previously got from the store) both the aggregates projectors run event by event reconstructing the two 
projections.
