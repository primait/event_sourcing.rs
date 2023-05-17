use async_trait::async_trait;
use sqlx::{Executor, Pool, Postgres};
use thiserror::Error;
use uuid::Uuid;

use esrs::postgres::{PgStore, PgStoreBuilder};
use esrs::{AggregateManager, AggregateState, Boxer, EventHandler, StoreEvent};

use crate::common::{new_pool, AggregateA, AggregateB, CommandA, CommandB, EventA, EventB};

#[path = "../common/lib.rs"]
mod common;

const CREATE_TABLE: &str = "CREATE TABLE IF NOT EXISTS shared_view \
    (shared_id uuid PRIMARY KEY NOT NULL, aggregate_id_a uuid, aggregate_id_b uuid, sum INTEGER)";

#[tokio::main]
async fn main() {
    let pool: Pool<Postgres> = new_pool().await;

    let _ = sqlx::query(CREATE_TABLE)
        .execute(&pool)
        .await
        .expect("Failed to create shared_view table");

    let shared_event_handler: Box<SharedEventHandler> = SharedEventHandler { pool: pool.clone() }.boxed();

    let store_a: PgStore<AggregateA> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(shared_event_handler.clone())
        .try_build()
        .await
        .expect("Failed to build PgStore for AggregateA");

    let store_b: PgStore<AggregateB> = PgStoreBuilder::new(pool.clone())
        .add_event_handler(shared_event_handler)
        .try_build()
        .await
        .expect("Failed to build PgStore for AggregateB");

    let shared_id: Uuid = Uuid::new_v4();

    let state_a: AggregateState<i32> = AggregateState::new();
    let aggregate_id_a: Uuid = *state_a.id();
    AggregateManager::new(store_a.boxed())
        .handle_command(state_a, CommandA { v: 5, shared_id })
        .await
        .expect("Failed to handle command for aggregate A");

    let state_b: AggregateState<i32> = AggregateState::new();
    let aggregate_id_b: Uuid = *state_b.id();
    AggregateManager::new(store_b.boxed())
        .handle_command(state_b, CommandB { v: 7, shared_id })
        .await
        .expect("Failed to handle command for aggregate A");

    let shared_view = SharedView::by_id(shared_id, &pool)
        .await
        .expect("Failed to get shared_view row by aggregate id A")
        .expect("Shared view not found by shared id");

    assert_eq!(shared_view.shared_id, shared_id);
    assert_eq!(shared_view.aggregate_id_a, Some(aggregate_id_a));
    assert_eq!(shared_view.aggregate_id_b, Some(aggregate_id_b));
    assert_eq!(shared_view.sum, 12); // 5 + 7

    dbg!(shared_view);
}

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Sql(#[from] sqlx::Error),
    #[error(transparent)]
    Json(#[from] serde_json::Error),
}

#[derive(Clone)]
pub struct SharedEventHandler {
    pool: Pool<Postgres>,
}

#[async_trait]
impl EventHandler<AggregateA> for SharedEventHandler {
    async fn handle(&self, event: &StoreEvent<EventA>) {
        let result = SharedView::upsert(
            Upsert::A {
                shared_id: event.payload.shared_id,
                aggregate_id: event.aggregate_id,
                value: event.payload.v,
            },
            &self.pool,
        )
        .await;

        if let Err(e) = result {
            println!("Error inserting A to shared view: {:?}", e)
        }
    }
}

#[async_trait]
impl EventHandler<AggregateB> for SharedEventHandler {
    async fn handle(&self, event: &StoreEvent<EventB>) {
        let result = SharedView::upsert(
            Upsert::B {
                shared_id: event.payload.shared_id,
                aggregate_id: event.aggregate_id,
                value: event.payload.v,
            },
            &self.pool,
        )
        .await;

        if let Err(e) = result {
            println!("Error inserting B to shared view: {:?}", e)
        }
    }
}

#[derive(sqlx::FromRow, Debug)]
pub struct SharedView {
    shared_id: Uuid,
    aggregate_id_a: Option<Uuid>,
    aggregate_id_b: Option<Uuid>,
    sum: i32,
}

pub enum Upsert {
    A {
        shared_id: Uuid,
        aggregate_id: Uuid,
        value: i32,
    },
    B {
        shared_id: Uuid,
        aggregate_id: Uuid,
        value: i32,
    },
}

impl SharedView {
    pub async fn by_id(
        shared_id: Uuid,
        executor: impl Executor<'_, Database = Postgres>,
    ) -> Result<Option<Self>, sqlx::Error> {
        sqlx::query_as::<_, Self>("SELECT * FROM shared_view WHERE shared_id = $1")
            .bind(shared_id)
            .fetch_optional(executor)
            .await
    }

    pub async fn upsert(ups: Upsert, executor: impl Executor<'_, Database = Postgres>) -> Result<(), sqlx::Error> {
        match ups {
            Upsert::A {
                shared_id,
                aggregate_id,
                value,
            } => upsert(shared_id, aggregate_id, value, "aggregate_id_a", executor).await,
            Upsert::B {
                shared_id,
                aggregate_id,
                value,
            } => upsert(shared_id, aggregate_id, value, "aggregate_id_b", executor).await,
        }
    }
}

async fn upsert(
    shared_id: Uuid,
    aggregate_id: Uuid,
    value: i32,
    id_field: &str,
    executor: impl Executor<'_, Database = Postgres>,
) -> Result<(), sqlx::Error> {
    let query = format!(
        "INSERT INTO shared_view (shared_id, {0}, sum) VALUES ($1, $2, $3) \
        ON CONFLICT (shared_id) DO UPDATE SET {0} = $2, sum = shared_view.sum + $3;",
        id_field
    );

    sqlx::query(query.as_str())
        .bind(shared_id)
        .bind(aggregate_id)
        .bind(value)
        .fetch_optional(executor)
        .await
        .map(|_| ())
}
