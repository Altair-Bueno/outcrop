use core::fmt::Debug;
use core::hash::Hash;

/// The signals a [`Formation`](crate::Formation) is made of.
///
/// Implemented by the strata built with
/// [`Formation::input`](crate::Formation::input), `with` and `route`, and by
/// the types declared with `strata!`. Each signal owns one change bit in
/// [`Flags`](Self::Flags).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a stack of strata",
    note = "strata are built with `Formation::input`, `with` and `route`, or declared with `strata!`"
)]
pub trait Strata {
    /// The change bits, one per signal.
    type Flags: Copy + Eq + Hash + Debug + Default;

    /// Flags marking every signal as changed, so that the first
    /// [`poll`](crate::Formation::poll) reports every initial value.
    const CHANGED: Self::Flags;

    /// Recomputes the derived signals whose dependencies changed, and marks
    /// those whose value changed.
    fn evaluate(&mut self, changed: &mut Self::Flags);
}

/// Strata accepting a `T`, either as an input or through a router.
///
/// The index `I` is inferred, and only tells apart where the `T` goes.
#[diagnostic::on_unimplemented(
    message = "`{T}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait Feed<T, I>: Strata {
    /// Stores or routes `value`, marking the inputs whose value changed.
    fn feed(&mut self, changed: &mut Self::Flags, value: T);
}

/// Strata whose every signal can be reported to the visitor `V`.
pub trait Report<V>: Strata {
    /// Reports every signal marked in `changed` to `visitor`.
    fn report(&self, changed: &Self::Flags, visitor: &mut V);
}
