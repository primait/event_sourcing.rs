//! These example demonstrates several ways a system can evolve through the use of the Schema
//! mechanism.
//!
//! - The first example demonstrates how to add a schema to an existing store.
//! - The second example demonstrates how to use the schema mechanism to deprecate certain event
//!   types.
//! - The third example demonstrates how to use the schema mechanism to upcast events.

mod adding_schema;
#[path = "../common/lib.rs"]
mod common;
mod deprecating_events;
mod upcasting;

use crate::common::util::new_pool;

#[tokio::main]
async fn main() {
    let pool = new_pool().await;
    adding_schema::example(pool.clone()).await;
    deprecating_events::example(pool.clone()).await;
    upcasting::example(pool).await;
}
