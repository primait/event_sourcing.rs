//! An [`AggregateView`] wraps an aggregate state and provides access to
//! information about the aggregate itself, such as its aggregate id.
//!
//! Aggregate views are primarily used by aggregate implementations when
//! handling commands and applying events. They allow business logic to
//! access aggregate metadata transparently.
//!
//! The wrapped state can be accessed through the standard dereferencing traits.
use std::ops::{Deref, DerefMut};
use uuid::Uuid;

/// A view over an aggregate state and its metadata.
///
/// This type combines an aggregate state with information about the
/// aggregate that owns it, such as its aggregate id.
///
/// Aggregate implementations receive an `AggregateView` when handling
/// commands or applying events, allowing business logic to inspect
/// aggregate metadata while interacting with the wrapped state as if
/// it were the state itself.
///
/// # Ownership
///
/// `AggregateView` is generic over the wrapped value and can therefore
/// represent either borrowed or owned state:
///
/// - `AggregateView<&T>` provides shared access to a state.
/// - `AggregateView<T>` owns the state.
///
/// The wrapped value can be accessed transparently through the standard
/// dereferencing traits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AggregateView<S> {
    aggregate_id: Uuid,
    inner: S,
}

impl<S> AggregateView<S> {
    /// Creates a new aggregate view.
    ///
    /// The supplied aggregate id identifies the aggregate that owns the
    /// wrapped state.    
    pub const fn new(id: Uuid, inner: S) -> Self {
        Self {
            aggregate_id: id,
            inner,
        }
    }

    /// Returns the id of the aggregate that owns the wrapped state.
    pub const fn id(&self) -> &Uuid {
        &self.aggregate_id
    }

    /// Consumes the view and returns the wrapped state.
    pub fn into_inner(self) -> S {
        self.inner
    }
}

/// Dereferences to the wrapped state.
///
/// This allows an `AggregateView<T>` to be used in most places where `T` is expected.
impl<S> Deref for AggregateView<S> {
    type Target = S;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

/// Mutably dereferences to the wrapped state.
///
/// This allows mutation of the wrapped state when the underlying value
/// supports mutable access.
impl<S> DerefMut for AggregateView<S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}
