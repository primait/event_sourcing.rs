use futures::StreamExt;
use sqlx::migrate::MigrateDatabase;
use sqlx::{pool::PoolOptions, PgConnection, Pool, Postgres, Transaction};
use uuid::Uuid;

use aggregate_merging::aggregates::{AggregateA, AggregateB};
use aggregate_merging::projectors::Counter;
use aggregate_merging::structs::{CommandA, CommandB, CounterError, EventA, EventB};
use esrs::postgres::Projector;
use esrs::{AggregateManager, StoreEvent};

// A simple example demonstrating rebuilding a read-side projection from an event
// stream
#[tokio::main]
async fn main() {
    let database_url: String = std::env::var("DATABASE_URL").expect("DATABASE_URL variable not set");

    Postgres::drop_database(database_url.as_str()).await.unwrap();
    Postgres::create_database(database_url.as_str()).await.unwrap();

    let pool: Pool<Postgres> = PoolOptions::new()
        .connect(database_url.as_str())
        .await
        .expect("Failed to create pool");
    let counter_id = Uuid::new_v4();

    setup(&pool, counter_id).await;

    let store_a = AggregateA::new(&pool).await.unwrap().event_store;
    let store_b = AggregateB::new(&pool).await.unwrap().event_store;

    let mut events_a = store_a.get_all();
    let mut events_b = store_b.get_all();

    let mut event_a_opt: Option<Result<StoreEvent<EventA>, CounterError>> = events_a.next().await;
    let mut event_b_opt: Option<Result<StoreEvent<EventB>, CounterError>> = events_b.next().await;

    let mut transaction: Transaction<Postgres> = pool.begin().await.expect("Failed to create transaction");

    loop {
        let a_opt: Option<&StoreEvent<EventA>> = event_a_opt.as_ref().map(|v| v.as_ref().unwrap());
        let b_opt: Option<&StoreEvent<EventB>> = event_b_opt.as_ref().map(|v| v.as_ref().unwrap());
        
        match (a_opt, b_opt) {
            (Some(a), Some(b)) if a.occurred_on <= b.occurred_on => {
                delete_and_project(store_a.projectors(), a, &mut *transaction)
                    .await
                    .expect("Failed to delete/project event");

                event_a_opt = events_a.next().await;
            }
            (Some(a), None) => {
                delete_and_project(store_a.projectors(), a, &mut *transaction)
                    .await
                    .expect("Failed to delete/project event");

                event_a_opt = events_a.next().await;
            }
            (Some(_), Some(b)) | (None, Some(b)) => {
                delete_and_project(store_b.projectors(), b, &mut *transaction)
                    .await
                    .expect("Failed to delete/project event");

                event_b_opt = events_b.next().await;
            }
            (None, None) => break,
        };
    }

    transaction.commit().await.unwrap();

    let counter = Counter::by_id(counter_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");

    assert_eq!(counter.count_b, 1);
    assert_eq!(counter.count_a, 1);
}

async fn setup(pool: &Pool<Postgres>, shared_id: Uuid) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");

    // Construct the two aggregates
    let agg_a = AggregateA::new(pool).await.expect("Failed to construct aggregate");
    let agg_b = AggregateB::new(pool).await.expect("Failed to construct aggregate");

    // Increment each count once
    let _ = agg_a
        .handle_command(Default::default(), CommandA::Inner { shared_id })
        .await
        .expect("Failed to handle command a");

    let _ = agg_b
        .handle_command(Default::default(), CommandB::Inner { shared_id })
        .await
        .expect("Failed to handle command b");
}

async fn delete_and_project<T: AggregateManager>(
    projectors: &[Box<dyn Projector<T> + Send + Sync>],
    event: &StoreEvent<T::Event>,
    transaction: &mut PgConnection,
) -> Result<(), T::Error> {
    for projector in projectors {
        projector.delete(event.aggregate_id, transaction).await?;
        projector.project(event, transaction).await?;
    }
    Ok(())
}
