mod esrs;

pub mod aggregate {
    pub use crate::esrs::aggregate::{Aggregate, AggregateManager, Eraser};
    pub use crate::esrs::state::AggregateState;
}

pub mod error {
    pub use serde_json::Error as JsonError;
    #[cfg(feature = "postgres")]
    pub use sqlx::Error as SqlxError;
}

pub mod policy {
    pub use crate::esrs::policy::Policy;
}

pub mod projector {
    pub use crate::esrs::projector::Projector;
}

pub mod store {
    #[cfg(feature = "postgres")]
    pub use crate::esrs::store::postgres::PgStore;
    pub use crate::esrs::store::{EventStore, StoreEvent};
}

pub mod types {
    pub use crate::esrs::SequenceNumber;
}
