#[cfg(feature = "postgres")]
pub use sqlx;

mod esrs;

pub mod aggregate {
    pub use crate::esrs::aggregate::{Aggregate, AggregateName};
    pub use crate::esrs::state::AggregateState;
}

pub mod error {
    pub use serde_json::Error as JsonError;
    #[cfg(feature = "postgres")]
    pub use sqlx::Error as SqlxError;
}

#[cfg(feature = "postgres")]
pub mod policy {
    pub use crate::esrs::postgres::policy::PgPolicy;
}

#[cfg(feature = "postgres")]
pub mod projector {
    pub use crate::esrs::postgres::projector::PgProjector;
}

pub mod store {
    #[cfg(feature = "postgres")]
    pub use crate::esrs::postgres::PgStore;
    pub use crate::esrs::store::{EventStore, ProjectEvent, StoreEvent};
}

pub mod types {
    pub use crate::esrs::SequenceNumber;
}
