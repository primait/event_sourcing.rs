pub mod aggregate;

#[cfg(feature = "postgres")]
mod postgres;

#[cfg(feature = "rabbit")]
mod rabbit;

#[cfg(feature = "kafka")]
mod kafka;
