use aggregate_merging::aggregates::{AggregateA, AggregateB};
use aggregate_merging::projectors::{Counter, CounterProjector};
use aggregate_merging::structs::{CommandA, CommandB, CounterError, ProjectorEvent};
use esrs::aggregate::{AggregateManager, AggregateState};
use esrs::projector::SqliteProjector;

use futures::StreamExt;
use sqlx::{pool::PoolOptions, Pool, Sqlite};
use uuid::Uuid;

// A simple example demonstrating rebuilding a read-side projection from an event
// stream
#[tokio::main]
async fn main() {
    let pool: Pool<Sqlite> = PoolOptions::new()
        .connect("sqlite::memory:")
        .await
        .expect("Failed to create pool");

    let count_id = Uuid::new_v4();

    setup(&pool, count_id.clone()).await;
    let agg_a = AggregateA::new(&pool).await.unwrap();
    let agg_b = AggregateB::new(&pool).await.unwrap();
    let store_a = agg_a.event_store();
    let store_b = agg_b.event_store();

    let mut events_a = store_a.get_all().map(|r| match r {
        Ok(e) => Ok(e.map(ProjectorEvent::from)),
        Err(e) => Err(e),
    });
    let mut events_b = store_b.get_all().map(|r| match r {
        Ok(e) => Ok(e.map(ProjectorEvent::from)),
        Err(e) => Err(e),
    });

    let projectors: Vec<Box<dyn SqliteProjector<ProjectorEvent, CounterError>>> = vec![Box::new(CounterProjector)];

    let mut a = None;
    let mut b = None;
    loop {
        if a.is_none() {
            a = events_a.next().await;
        }
        if b.is_none() {
            b = events_b.next().await;
        }
        if a.is_none() && b.is_none() {
            break;
        }
        let mut connection = pool.acquire().await.unwrap();
        let _ = sqlx::query("BEGIN").execute(&mut connection).await.unwrap();
        for projector in &projectors {
            if a.is_none() {
                projector
                    .project(b.as_ref().unwrap().as_ref().unwrap(), &mut connection)
                    .await
                    .unwrap();
                b = None;
                continue;
            }
            if b.is_none() {
                projector
                    .project(&a.as_ref().unwrap().as_ref().unwrap(), &mut connection)
                    .await
                    .unwrap();
                a = None;
                continue;
            }
            let a_inner = a.as_ref().unwrap().as_ref().unwrap();
            let b_inner = b.as_ref().unwrap().as_ref().unwrap();
            if a_inner.occurred_on > b_inner.occurred_on {
                projector.project(b_inner, &mut connection).await.unwrap();
                b = None;
            } else {
                projector.project(a_inner, &mut connection).await.unwrap();
                a = None;
            }
        }
        let _ = sqlx::query("COMMIT").execute(&mut connection).await.unwrap();
    }

    let counter = Counter::by_id(count_id, &pool)
        .await
        .expect("Failed to retrieve counter")
        .expect("Failed to find counter");
    assert!(counter.count_a == 1 && counter.count_b == 1);
}

async fn setup(pool: &Pool<Sqlite>, count_id: Uuid) {
    let () = sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");

    // Construct the two aggregates
    let agg_a = AggregateA::new(pool).await.expect("Failed to construct aggregate");
    let a_state = AggregateState::new(count_id);

    let agg_b = AggregateB::new(pool).await.expect("Failed to construct aggregate");
    let b_state = AggregateState::new(count_id);

    // Increment each count once
    let _ = agg_a
        .handle_command(a_state, CommandA::Inner)
        .await
        .expect("Failed to handle command a");

    let _ = agg_b
        .handle_command(b_state, CommandB::Inner)
        .await
        .expect("Failed to handle command b");

    //Drop and rebuild the counters projection table
    sqlx::query("DROP TABLE counters")
        .execute(pool)
        .await
        .expect("Failed to drop table");
    sqlx::query("CREATE TABLE counters (\"counter_id\" UUID PRIMARY KEY NOT NULL, \"count_a\" INTEGER NOT NULL, \"count_b\" INTEGER NOT NULL );")
        .execute(pool)
        .await
        .expect("Failed to recreate counters table");
}