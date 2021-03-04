#[cfg(feature = "async")]
pub use async_impl::*;

#[cfg(feature = "async")]
mod async_impl;

#[cfg(feature = "blocking")]
pub mod blocking;

mod query;
pub mod state;

pub type SequenceNumber = i32;

pub struct StoreParams<'a> {
    pub host: &'a str,
    pub port: Option<&'a str>,
    pub user: &'a str,
    pub pass: &'a str,
    pub schema: &'a str,
}

impl<'a> StoreParams<'a> {
    pub fn postgres_url(&self) -> String {
        format!(
            "postgres://{}:{}@{}:{}/{}",
            &self.user,
            &self.pass,
            &self.host,
            &self.port.unwrap_or("5432"),
            &self.schema
        )
    }
}
