//! Basic example showing how to rebuild with two different strategies:
//!
//! - by aggregate id: the rebuild is made opening a transaction for every id fetched by a pre-made
//!   query. In that transaction every row having a matching aggregate id is deleted and then
//!   reprojected.
//!
//! - all at once: this is done in a transaction truncating all the table content and then rebuilding
//!   all the events retrieved in the moment the transaction is opened.
//!
//! Note that is not possible to rebuild using non-replayable event handlers.
//!
//! This will not compile:
//!
//! ```rust
//! let rebuilder: PgRebuilder<BasicAggregate> = PgRebuilder::new().with_event_handlers(vec![Box::new(AnotherEventHandler)]);
//! ```
//!
//! The output:
//!
//! ```shell
//! error[E0277]: the trait bound `AnotherEventHandler: ReplayableEventHandler<basic::BasicAggregate>` is not satisfied
//!   --> examples/rebuilder/main.rs
//!    |
//! 54 |         PgRebuilder::new().with_event_handlers(vec![Box::new(handler_v2), Box::new(AnotherEventHandler)]);
//!    |                                                                           ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `ReplayableEventHandler<basic::BasicAggregate>` is not implemented for `AnotherEventHandler`
//!    |
//!    = help: the following other types implement trait `ReplayableEventHandler<A>`:
//!              BasicEventHandlerV1
//!              BasicEventHandlerV2
//!    = note: required for the cast from `AnotherEventHandler` to the object type `dyn ReplayableEventHandler<basic::BasicAggregate> + Send`
//! ```

use sqlx::{Pool, Postgres};
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::rebuilder::{PgRebuilder, Rebuilder};
use esrs::{AggregateManager, AggregateState};

use crate::common::{new_pool, BasicAggregate, BasicCommand, BasicView};
use crate::event_handler::{AnotherEventHandler, BasicEventHandlerV1, BasicEventHandlerV2};
use crate::transactional_event_handler::{BasicTransactionalEventHandlerV1, BasicTransactionalEventHandlerV2};

#[path = "../common/lib.rs"]
mod common;
mod event_handler;
mod transactional_event_handler;

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    // Rebuilder::by_aggregate_id rebuilding
    let view: BasicView = BasicView::new("view_v1", &pool).await;
    let transactional_view: BasicView = BasicView::new("transactional_view_v1", &pool).await;

    let aggregate_id: Uuid = Uuid::new_v4();
    setup(aggregate_id, view.clone(), transactional_view.clone(), pool.clone()).await;
    rebuild_by_aggregate_id(aggregate_id, view.clone(), transactional_view.clone(), pool.clone()).await;

    // Rebuilder::all_at_once rebuilding
    let view: BasicView = BasicView::new("view_v2", &pool).await;
    let transactional_view: BasicView = BasicView::new("transactional_view_v2", &pool).await;

    let aggregate_id: Uuid = Uuid::new_v4();
    setup(aggregate_id, view.clone(), transactional_view.clone(), pool.clone()).await;
    rebuild_all_at_once(aggregate_id, view.clone(), transactional_view.clone(), pool.clone()).await;
}

async fn setup(aggregate_id: Uuid, view: BasicView, transactional_view: BasicView, pool: Pool<Postgres>) {
    let pg_store: PgStore<BasicAggregate> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(BasicEventHandlerV1 {
            pool: pool.clone(),
            view: view.clone(),
        })
        .add_event_handler(AnotherEventHandler)
        .add_transactional_event_handler(BasicTransactionalEventHandlerV1 {
            view: transactional_view.clone(),
        })
        .try_build()
        .await
        .unwrap();

    let manager: AggregateManager<BasicAggregate> = AggregateManager::new(pg_store);
    manager
        .handle_command(
            AggregateState::with_id(aggregate_id),
            BasicCommand {
                content: "basic_command".to_string(),
            },
        )
        .await
        .unwrap();

    assert_eq!(
        view.by_id(aggregate_id, &pool).await.unwrap().unwrap().content,
        "basic_command.v1"
    );

    assert_eq!(
        transactional_view
            .by_id(aggregate_id, &pool)
            .await
            .unwrap()
            .unwrap()
            .content,
        "basic_command.v1"
    );
}

async fn rebuild_by_aggregate_id(
    aggregate_id: Uuid,
    view: BasicView,
    transactional_view: BasicView,
    pool: Pool<Postgres>,
) {
    let event_handler_v2 = BasicEventHandlerV2 {
        pool: pool.clone(),
        view: view.clone(),
    };

    let transactional_event_handler_v2 = BasicTransactionalEventHandlerV2 {
        view: transactional_view.clone(),
    };

    let rebuilder: PgRebuilder<BasicAggregate> = PgRebuilder::new()
        .with_event_handlers(vec![Box::new(event_handler_v2)])
        .with_transactional_event_handlers(vec![Box::new(transactional_event_handler_v2)]);

    rebuilder.by_aggregate_id(pool.clone()).await.unwrap();

    assert_eq!(
        view.by_id(aggregate_id, &pool).await.unwrap().unwrap().content,
        "basic_command.v2"
    );

    assert_eq!(
        transactional_view
            .by_id(aggregate_id, &pool)
            .await
            .unwrap()
            .unwrap()
            .content,
        "basic_command.v2"
    );
}

async fn rebuild_all_at_once(aggregate_id: Uuid, view: BasicView, transactional_view: BasicView, pool: Pool<Postgres>) {
    let event_handler_v2 = BasicEventHandlerV2 {
        pool: pool.clone(),
        view: view.clone(),
    };

    let transactional_event_handler_v2 = BasicTransactionalEventHandlerV2 {
        view: transactional_view.clone(),
    };

    let rebuilder: PgRebuilder<BasicAggregate> = PgRebuilder::new()
        .with_event_handlers(vec![Box::new(event_handler_v2)])
        .with_transactional_event_handlers(vec![Box::new(transactional_event_handler_v2)]);

    rebuilder.all_at_once(pool.clone()).await.unwrap();

    assert_eq!(
        view.by_id(aggregate_id, &pool).await.unwrap().unwrap().content,
        "basic_command.v2"
    );

    assert_eq!(
        transactional_view
            .by_id(aggregate_id, &pool)
            .await
            .unwrap()
            .unwrap()
            .content,
        "basic_command.v2"
    );
}
