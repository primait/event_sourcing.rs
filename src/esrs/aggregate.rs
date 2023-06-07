/// The Aggregate trait is responsible for validating commands, mapping commands to events, and applying
/// events onto the state.
///
/// An Aggregate should be able to derive its own state from nothing but its initial configuration, and its
/// event stream. Applying the same events, in the same order, to the same aggregate, should always yield an
/// identical aggregate state.
///
/// This trait is purposefully _synchronous_. If you are implementing this trait, your aggregate
/// should not have any side effects. If you need additional information to handle commands correctly, then
/// consider looking up that information and placing it in the command.
pub trait Aggregate {
    /// The `NAME` const is responsible for naming an aggregate type.
    /// Each aggregate type should have a name that is unique among all the aggregate types in your application.
    ///
    /// Aggregates are linked to their instances & events using their `NAME` and their `aggregate_id`.
    /// Be very careful when changing `NAME`, as doing so will break the link between all the aggregates
    /// of their type, and their events!
    const NAME: &'static str;

    /// Internal aggregate state. This will be wrapped in [`AggregateState`] and could be used to validate
    /// commands.
    type State: Default + Send;

    /// A command is an action that the caller can execute over an aggregate in order to let it emit
    /// an event.
    type Command: Send;

    /// An event represents a fact that took place in the domain. They are the source of truth;
    /// your current state is derived from the events.
    type Event;

    /// This associated type is used to get domain errors while handling a command.
    type Error: std::error::Error;

    /// Handles, validate a command and emits events.
    ///
    /// # Errors
    ///
    /// Will return `Err` if the user of this library set up command validations. Every error here
    /// could be just a "domain error". No technical errors.
    fn handle_command(state: &Self::State, command: Self::Command) -> Result<Vec<Self::Event>, Self::Error>;

    /// Updates the aggregate state using the new event. This assumes that the event can be correctly applied
    /// to the state.
    ///
    /// If this is not the case, this function is allowed to panic.
    fn apply_event(state: Self::State, payload: Self::Event) -> Self::State;
}
