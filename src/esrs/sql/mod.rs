pub mod event;
pub mod migrations;
pub mod statements;

#[macro_export]
macro_rules! statement {
    ($file:expr, $ty:ty $(,)?) => {{
        format!(include_str!($file), format!("{}_events", <$ty>::NAME))
    }};
}
