use async_trait::async_trait;

#[cfg(all(feature = "rebuilder", feature = "postgres"))]
pub use pg_rebuilder::PgRebuilder;

use crate::Aggregate;

#[cfg(all(feature = "rebuilder", feature = "postgres"))]
mod pg_rebuilder;

#[async_trait]
pub trait Rebuilder<A>
where
    A: Aggregate,
{
    type Executor;
    type Error: std::error::Error;

    async fn by_aggregate_id(&self, executor: Self::Executor) -> Result<(), Self::Error>;
    async fn all_at_once(&self, executor: Self::Executor) -> Result<(), Self::Error>;
}
