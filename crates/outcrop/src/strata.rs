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
///
/// # Implementing it by hand
///
/// `Strata`, [`Feed`], [`Get`] and [`Report`] are open because `strata!`
/// implements them in the crate that declares the struct, so they can also be
/// implemented by hand. [`Formation`](crate::Formation) relies on such an
/// impl keeping these rules, which `strata!` and the builder keep:
///
/// - Every signal owns exactly one bit of `Flags`, and `Flags::default()` marks
///   none.
/// - [`CHANGED`](Self::CHANGED) marks every signal.
/// - [`evaluate`](Self::evaluate) visits the signals bottom-up, so each sees
///   the bits set by those beneath it. It recomputes a memo only if a
///   dependency's bit is set, marks the memo only if its value changed, marks a
///   view whenever a dependency's bit is set, and never clears a bit.
/// - [`Feed::feed`] marks an input only if the value differs from the stored
///   one, and evaluates nothing.
/// - [`Get::get`] is [`Stale`](Reading::Stale) exactly when the signal is a
///   memo, or is computed from one, and any bit of `Flags` is set.
/// - [`Report::report`] visits exactly the marked signals, bottom-up, and
///   under `panic = "unwind"` clears each bit just before visiting its signal;
///   under `panic = "abort"` it may leave the bits for
///   [`Formation::poll`](crate::Formation::poll) to reset.
///
/// An impl breaking them makes its own formations report or compute wrong
/// values; it cannot cause undefined behaviour, as the crate has no unsafe
/// code.
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

    /// Returns the current value of `T`, [`Stale`](Reading::Stale) if it is
    /// or depends on a memo and any signal is marked in `changed`.
    fn get(&self, changed: &Self::Flags) -> Reading<Self::Ref<'_>>;
}

/// Strata whose every signal can be reported to the visitor `V`.
pub trait Report<V>: Strata {
    /// Reports every signal marked in `changed` to `visitor`, clearing the
    /// bit of each signal before visiting it.
    fn report(&self, changed: &mut Self::Flags, visitor: &mut V);
}
