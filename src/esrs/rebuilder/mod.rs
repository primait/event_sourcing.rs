use async_trait::async_trait;

#[cfg(all(feature = "rebuilder", feature = "postgres"))]
pub use pg_rebuilder::PgRebuilder;

use crate::Aggregate;

#[cfg(all(feature = "rebuilder", feature = "postgres"))]
mod pg_rebuilder;

#[async_trait]
pub trait Rebuilder<A, E>
where
    A: Aggregate,
{
    async fn by_aggregate_id(&self, executor: E) -> Result<(), A::Error>;
    async fn all_at_once(&self, executor: E) -> Result<(), A::Error>;
}
