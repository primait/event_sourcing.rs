use futures::StreamExt;
use sqlx::{PgConnection, Pool, Postgres, Transaction};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState, ReplayableEventHandler, StoreEvent, TransactionalEventHandler};

use crate::common::{
    new_pool, AggregateA, AggregateB, CommandA, CommandB, CommonError, EventA, EventB, SharedEventHandler, SharedView,
};
use crate::transactional_event_handler::SharedTransactionalEventHandler;

#[path = "../common/lib.rs"]
mod common;
mod transactional_event_handler;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let shared_id: Uuid = Uuid::new_v4();
    let view: SharedView = SharedView::new("view", &pool).await;
    let transactional_view: SharedView = SharedView::new("transactional_view", &pool).await;

    setup(shared_id, pool.clone(), view.clone(), transactional_view.clone()).await;

    rebuild_multi_aggregate(shared_id, pool, view, transactional_view).await
}

async fn rebuild_multi_aggregate(
    shared_id: Uuid,
    pool: Pool<Postgres>,
    view: SharedView,
    transactional_view: SharedView,
) {
    let store_a: PgStore<AggregateA> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();
    let store_b: PgStore<AggregateB> = PgStoreBuilder::new(pool.clone()).try_build().await.unwrap();

    let event_handler = Box::new(SharedEventHandler {
        pool: pool.clone(),
        view: view.clone(),
    });

    let transactional_event_handler = Box::new(SharedTransactionalEventHandler {
        view: transactional_view.clone(),
    });

    // It is important to have `ReplayableEventHandler`s only.
    let event_handlers_a: Vec<Box<dyn ReplayableEventHandler<AggregateA>>> = vec![event_handler.clone()];
    let event_handlers_b: Vec<Box<dyn ReplayableEventHandler<AggregateB>>> = vec![event_handler.clone()];

    let transactional_event_handlers_a: Vec<Box<dyn TransactionalEventHandler<AggregateA, PgConnection>>> =
        vec![transactional_event_handler.clone()];
    let transactional_event_handlers_b: Vec<Box<dyn TransactionalEventHandler<AggregateB, PgConnection>>> =
        vec![transactional_event_handler.clone()];

    let mut events_a = store_a.stream_events(&pool);
    let mut events_b = store_b.stream_events(&pool);

    // Fetch first element of both the tables
    let mut event_a_opt: Option<Result<StoreEvent<EventA>, CommonError>> = events_a.next().await;
    let mut event_b_opt: Option<Result<StoreEvent<EventB>, CommonError>> = events_b.next().await;

    // At this point it's possible to open a transaction
    let mut transaction: Transaction<Postgres> = pool.begin().await.unwrap();

    // There are 3 choices here:
    // - Truncate all the tables where the event handlers and transactional event handlers insist on.
    // - Implement the EventHandler::delete and TransactionalEventHandler::delete functions
    // - Implement both the EventHandler and TransactionalEventHandler function upserting instead of
    //   inserting values and updating them in two steps.
    //
    // In this example we truncate the tables

    let query: String = format!("TRUNCATE TABLE {}", view.table_name());
    let _ = sqlx::query(query.as_str()).execute(&pool).await.unwrap();

    let query: String = format!("TRUNCATE TABLE {}", transactional_view.table_name());
    let _ = sqlx::query(query.as_str()).execute(&mut *transaction).await.unwrap();

    loop {
        let a_opt: Option<&StoreEvent<EventA>> = event_a_opt.as_ref().map(|v| v.as_ref().unwrap());
        let b_opt: Option<&StoreEvent<EventB>> = event_b_opt.as_ref().map(|v| v.as_ref().unwrap());

        match (a_opt, b_opt) {
            // If both the streams returned a value we check what's the oldest. If the oldest is `a`
            // we proceed to run the transactional event handlers from AggregateA.
            (Some(a), Some(b)) if a.occurred_on <= b.occurred_on => {
                for transactional_event_handler in &transactional_event_handlers_a {
                    transactional_event_handler.handle(a, &mut transaction).await.unwrap();
                }
                for event_handler in &event_handlers_a {
                    event_handler.handle(a).await;
                }

                // Get next value from AggregateA events stream
                event_a_opt = events_a.next().await;
            }
            // If only the stream on AggregateA events contains values we proceed to run the projectors
            // from AggregateA.
            (Some(a), None) => {
                for transactional_event_handler in &transactional_event_handlers_a {
                    transactional_event_handler.handle(a, &mut transaction).await.unwrap();
                }
                for event_handler in &event_handlers_a {
                    event_handler.handle(a).await;
                }

                // Get next value from AggregateA events stream
                event_a_opt = events_a.next().await;
            }
            // If both the streams returned a value and AggregateB event is older or if only the stream
            // on AggregateB events contains values we proceed to run the projectors from AggregateB.
            (Some(_), Some(b)) | (None, Some(b)) => {
                for transactional_event_handler in &transactional_event_handlers_b {
                    transactional_event_handler.handle(b, &mut transaction).await.unwrap();
                }
                for event_handler in &event_handlers_b {
                    event_handler.handle(b).await;
                }

                // Get next value from AggregateB events stream
                event_b_opt = events_b.next().await;
            }
            // If both the streams are empty then we break the loop.
            (None, None) => break,
        };
    }

    // Finally commit the transaction
    transaction.commit().await.unwrap();

    // This fixed the amount that were stored as a negative value
    assert_eq!(view.by_id(shared_id, &pool).await.unwrap().unwrap().sum, 17);

    assert_eq!(
        transactional_view.by_id(shared_id, &pool).await.unwrap().unwrap().sum,
        17
    );
}

async fn setup(shared_id: Uuid, pool: Pool<Postgres>, view: SharedView, transactional_view: SharedView) {
    let pg_store_a: PgStore<AggregateA> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(SharedEventHandler {
            pool: pool.clone(),
            view: view.clone(),
        })
        .add_transactional_event_handler(SharedTransactionalEventHandler {
            view: transactional_view.clone(),
        })
        .try_build()
        .await
        .unwrap();

    let manager: AggregateManager<AggregateA> = AggregateManager::new(pg_store_a);
    manager
        .handle_command(AggregateState::default(), CommandA { v: 10, shared_id })
        .await
        .unwrap();

    let pg_store_b: PgStore<AggregateB> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(SharedEventHandler {
            pool: pool.clone(),
            view: view.clone(),
        })
        .add_transactional_event_handler(SharedTransactionalEventHandler {
            view: transactional_view.clone(),
        })
        .try_build()
        .await
        .unwrap();

    let manager: AggregateManager<AggregateB> = AggregateManager::new(pg_store_b);
    manager
        .handle_command(AggregateState::default(), CommandB { v: 7, shared_id })
        .await
        .unwrap();

    assert_eq!(view.by_id(shared_id, &pool).await.unwrap().unwrap().sum, 17);

    assert_eq!(
        transactional_view.by_id(shared_id, &pool).await.unwrap().unwrap().sum,
        17
    );

    // There was an error before. All amounts were written as a negative value
    view.update_sum(shared_id, -17, &pool).await.unwrap();
    transactional_view.update_sum(shared_id, -17, &pool).await.unwrap();

    assert_eq!(view.by_id(shared_id, &pool).await.unwrap().unwrap().sum, -17);

    assert_eq!(
        transactional_view.by_id(shared_id, &pool).await.unwrap().unwrap().sum,
        -17
    );
}
