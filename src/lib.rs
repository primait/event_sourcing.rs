pub use crate::esrs::aggregate::{Aggregate, AggregateManager};
pub use crate::esrs::policy::Policy;
pub use crate::esrs::state::AggregateState;
pub use crate::esrs::store::{EventStore, StoreEvent};

mod esrs;

#[cfg(feature = "postgres")]
pub mod postgres {
    pub use crate::esrs::postgres::projector::{Projector, Consistency};
    pub use crate::esrs::postgres::store::PgStore;
}

pub mod error {
    pub use serde_json::Error as JsonError;
    #[cfg(feature = "postgres")]
    pub use sqlx::Error as SqlxError;
}

pub mod types {
    pub use crate::esrs::SequenceNumber;
}
