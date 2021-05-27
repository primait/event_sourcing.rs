mod esrs;

pub mod aggregate {
    pub use crate::esrs::aggregate::{Aggregate, Eraser, Identifier};
    pub use crate::esrs::state::AggregateState;
}

pub mod error {
    pub use serde_json::Error as JsonError;
    #[cfg(any(feature = "postgres", feature = "sqlite"))]
    pub use sqlx::Error as SqlxError;
}

#[cfg(any(feature = "postgres", feature = "sqlite"))]
pub mod policy {
    #[cfg(feature = "postgres")]
    pub use crate::esrs::postgres::policy::PgPolicy;
    #[cfg(feature = "sqlite")]
    pub use crate::esrs::sqlite::policy::SqlitePolicy;
}

#[cfg(any(feature = "postgres", feature = "sqlite"))]
pub mod projector {
    #[cfg(feature = "postgres")]
    pub use crate::esrs::postgres::projector::{PgProjector, PgProjectorEraser};
    #[cfg(feature = "sqlite")]
    pub use crate::esrs::sqlite::projector::{SqliteProjector, SqliteProjectorEraser};
}

pub mod store {
    #[cfg(feature = "postgres")]
    pub use crate::esrs::postgres::PgStore;
    #[cfg(feature = "sqlite")]
    pub use crate::esrs::sqlite::SqliteStore;
    pub use crate::esrs::store::{EraserStore, EventStore, ProjectorStore, StoreEvent};
}

pub mod types {
    pub use crate::esrs::SequenceNumber;
}
