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

pub mod store {
    pub use crate::esrs::store::{EventStore, StoreEvent};

    #[cfg(feature = "postgres")]
    pub mod postgres {
        pub use crate::esrs::postgres::policy::Policy;
        pub use crate::esrs::postgres::projector::Projector;
        pub use crate::esrs::postgres::PgStore;
    }
}

pub mod types {
    pub use crate::esrs::SequenceNumber;
}
