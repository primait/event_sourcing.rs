pub mod aggregate {
    pub use esrs_core::aggregate::{Aggregate, AggregateName};
    pub use esrs_core::state::AggregateState;
}

pub mod error {
    pub use esrs_core::JsonError;
    #[cfg(feature = "postgres")]
    pub use esrs_core::sqlx::Error as SqlxError;
}

#[cfg(feature = "postgres")]
pub mod projector {
    pub use esrs_core::postgres::PgProjector;
}

pub mod policy {
    pub use esrs_core::policy::Policy;
}

#[cfg(feature = "postgres")]
pub mod sqlx {
    pub use esrs_core::sqlx::*;
}

pub mod store {
    #[cfg(feature = "postgres")]
    pub use esrs_core::postgres::PgStore;
    pub use esrs_core::store::{EventStore, ProjectEvent, StoreEvent};
}

pub mod types {
    pub use esrs_core::SequenceNumber;
}
