use core::borrow::Borrow;
use core::fmt::Debug;
use core::hash::Hash;

use crate::reading::Reading;

/// The signals a [`Formation`](crate::Formation) is made of.
///
/// Implemented by the strata built with
/// [`Formation::input`](crate::Formation::input), `memo`, `view` and `route`,
/// and by the types declared with `strata!`. Each signal owns one change bit in
/// [`Flags`](Self::Flags).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a stack of strata",
    note = "strata are built with `Formation::input`, `memo`, `view` and `route`, or declared with `strata!`"
)]
pub trait Strata {
    /// The change bits, one per signal.
    type Flags: Copy + Eq + Hash + Debug + Default;

    /// Flags marking every signal as changed, so that the first
    /// [`poll`](crate::Formation::poll) reports every initial value.
    const CHANGED: Self::Flags;

    /// Recomputes the memos whose dependencies changed, and marks the memos
    /// whose value changed and the views whose dependencies changed.
    fn evaluate(&mut self, changed: &mut Self::Flags);
}

/// Strata accepting a `T`, either as an input or through a router.
///
/// The index `I` is inferred, and only tells apart where the `T` goes.
#[diagnostic::on_unimplemented(
    message = "`{T}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; memos and views are computed by the formation and cannot be fed"
)]
pub trait Feed<T, I>: Strata {
    /// Stores or routes `value`, marking the inputs whose value changed.
    fn feed(&mut self, changed: &mut Self::Flags, value: T);
}

/// Strata containing the signal `T`.
///
/// The index `I` is inferred, and only tells apart where the `T` is.
#[diagnostic::on_unimplemented(
    message = "`{T}` is not a signal of this formation",
    label = "not an input, memo or view of this formation"
)]
pub trait Get<T, I>: Strata {
    /// How the value is handed out: a reference to a stored input or memo,
    /// or the value of a view, computed on the spot.
    type Ref<'a>: Borrow<T>
    where
        Self: 'a;

    /// Returns the current value of `T`, [`Stale`](Reading::Stale) if a
    /// memo it depends on may be recomputed by the next poll.
    fn get(&self, changed: &Self::Flags) -> Reading<Self::Ref<'_>>;
}

/// Strata whose every signal can be reported to the visitor `V`.
pub trait Report<V>: Strata {
    /// Reports every signal marked in `changed` to `visitor`.
    fn report(&self, changed: &Self::Flags, visitor: &mut V);
}
