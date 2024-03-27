use crate::sql::event::Persistable;

/// To support decoupling between the [`crate::Aggregate::Event`] type and the schema of the DB table
/// in [`super::PgStore`] you can create a schema type that implements [`Persistable`] and [`Schema`]
/// where `E = Aggregate::Event`.
///
/// Note: Although [`Schema::to_event`] returns an [`Option`] for any given event and implementation.
///
/// The following must hold
///
/// ```rust
/// # use serde::{Serialize, Deserialize};
/// # use esrs::store::postgres::Schema as SchemaTrait;
/// #
/// # #[derive(Clone, Eq, PartialEq, Debug)]
/// # struct Event {
/// #   a: u32,
/// # }
/// #
/// # #[derive(Serialize, Deserialize)]
/// # struct Schema {
/// #   a: u32,
/// # }
/// #
/// # #[cfg(feature = "upcasting")]
/// # impl esrs::sql::event::Upcaster for Schema {}
/// #
/// # impl SchemaTrait<Event> for Schema {
/// #   fn from_event(Event { a }: Event) -> Self {
/// #     Self { a }
/// #   }
/// #
/// #   fn to_event(self) -> Option<Event> {
/// #     Some(Event { a: self.a })
/// #   }
/// # }
/// #
/// # let event = Event { a: 42 };
/// assert_eq!(Some(event.clone()), Schema::from_event(event).to_event());
/// ```
pub trait Schema<E>: Persistable {
    /// Converts the event into the schema type.
    fn from_event(event: E) -> Self;

    /// Converts the schema into the event type.
    ///
    /// This returns an option to enable skipping deprecated event which are persisted in the DB.
    ///
    /// Note: Although this function returns an [`Option`] for any given event and implementation.
    ///
    /// The following must hold
    ///
    /// ```rust
    /// # use serde::{Serialize, Deserialize};
    /// # use esrs::store::postgres::Schema as SchemaTrait;
    /// #
    /// # #[derive(Clone, Eq, PartialEq, Debug)]
    /// # struct Event {
    /// #   a: u32,
    /// # }
    /// #
    /// # #[derive(Serialize, Deserialize)]
    /// # struct Schema {
    /// #   a: u32,
    /// # }
    /// #
    /// # #[cfg(feature = "upcasting")]
    /// # impl esrs::sql::event::Upcaster for Schema {}
    /// #
    /// # impl SchemaTrait<Event> for Schema {
    /// #   fn from_event(Event { a }: Event) -> Self {
    /// #     Self { a }
    /// #   }
    /// #
    /// #   fn to_event(self) -> Option<Event> {
    /// #     Some(Event { a: self.a })
    /// #   }
    /// # }
    /// #
    /// # let event = Event { a: 42 };
    /// assert_eq!(Some(event.clone()), Schema::from_event(event).to_event());
    /// ```
    fn to_event(self) -> Option<E>;
}

impl<E> Schema<E> for E
where
    E: Persistable,
{
    fn from_event(event: E) -> Self {
        event
    }
    fn to_event(self) -> Option<E> {
        Some(self)
    }
}
