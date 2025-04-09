use async_trait::async_trait;

#[cfg(feature = "postgres")]
pub use pg_rebuilder::PgRebuilder;
use uuid::Uuid;

use crate::Aggregate;

#[cfg(feature = "postgres")]
mod pg_rebuilder;

#[async_trait]
pub trait Rebuilder<A>
where
    A: Aggregate,
{
    type Executor;
    type Error: std::error::Error;

    async fn by_aggregate_id(&self, executor: Self::Executor) -> Result<(), Self::Error>;
    async fn just_one_aggregate(&self, aggregate_id: Uuid, executor: Self::Executor) -> Result<(), Self::Error>;
    async fn all_at_once(&self, executor: Self::Executor) -> Result<(), Self::Error>;
}
