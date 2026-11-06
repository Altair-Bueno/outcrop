//! The building blocks a [`Formation`](crate::Formation) is made of.
//!
//! A formation's strata form a type-level list: every stratum wraps the ones
//! declared before it, ending in [`Nil`]. These types and traits appear in the
//! formation's type and bounds; they are rarely written by hand.

use core::fmt;
use core::marker::PhantomData;

use crate::compute::Compute;
use crate::flags::Flags;
use crate::sink::Sink;
use crate::visit::Visit;

/// The empty bottom of every formation.
#[derive(Debug)]
pub struct Nil;

/// One stratum `E` deposited on top of the strata `Tail`.
#[derive(Debug)]
pub struct Stratum<E, Tail> {
    pub(super) entry: E,
    pub(super) tail: Tail,
}

/// An input signal fed by the caller.
#[derive(Debug)]
pub struct Input<T>(pub(super) T);

/// A signal derived from the strata beneath it by `F`.
pub struct Derived<F, M, O> {
    pub(super) f: F,
    pub(super) value: O,
    pub(super) marker: PhantomData<fn() -> M>,
}

impl<F, M, O: fmt::Debug> fmt::Debug for Derived<F, M, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Derived")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

/// A router that turns a fed `&In` into feeds of the strata beneath it.
pub struct Router<In: ?Sized, F> {
    pub(super) f: F,
    pub(super) marker: PhantomData<fn(&In)>,
}

impl<In: ?Sized, F> fmt::Debug for Router<In, F> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Router").finish_non_exhaustive()
    }
}

/// Type-level index of the topmost stratum.
#[derive(Debug)]
pub struct Here;

/// Type-level index of a stratum beneath the topmost one.
#[derive(Debug)]
pub struct There<I>(PhantomData<I>);

/// Type-level index past the bottom of the strata, where nothing is found.
#[derive(Debug)]
pub struct Nowhere;

mod sealed {
    #[expect(
        unnameable_types,
        reason = "sealed supertrait: unnameable so that `Absent` cannot be implemented downstream"
    )]
    pub trait Sealed {}
}

/// A value fed to a formation that has no input or router accepting it.
///
/// Never implemented. It exists so that feeding an unknown value reports
/// this trait's diagnostic instead of a type mismatch.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait Absent: sealed::Sealed {
    /// Consumes the value.
    fn absent(self);
}

/// Number of signals in the strata.
pub trait Len {
    /// The number of signals.
    const LEN: usize;
}

impl Len for Nil {
    const LEN: usize = 0;
}

impl<T, Tail: Len> Len for Stratum<Input<T>, Tail> {
    const LEN: usize = Tail::LEN + 1;
}

impl<F, M, O, Tail: Len> Len for Stratum<Derived<F, M, O>, Tail> {
    const LEN: usize = Tail::LEN + 1;
}

impl<In: ?Sized, F, Tail: Len> Len for Stratum<Router<In, F>, Tail> {
    const LEN: usize = Tail::LEN;
}

/// A stratum holding a signal value.
pub trait Value {
    /// The signal.
    type Signal;

    /// Returns the current value of the signal.
    fn value(&self) -> &Self::Signal;
}

impl<T> Value for Input<T> {
    type Signal = T;

    fn value(&self) -> &T {
        &self.0
    }
}

impl<F, M, O> Value for Derived<F, M, O> {
    type Signal = O;

    fn value(&self) -> &O {
        &self.value
    }
}

/// Finds the signal `T` at the type-level index `I`.
pub trait Select<T, I> {
    /// The signal's position, counted from the first stratum.
    const INDEX: usize;

    /// Returns the current value of the signal.
    fn select(&self) -> &T;
}

impl<E: Value, Tail: Len> Select<E::Signal, Here> for Stratum<E, Tail> {
    const INDEX: usize = Tail::LEN;

    fn select(&self) -> &E::Signal {
        self.entry.value()
    }
}

impl<T, I, E, Tail: Select<T, I>> Select<T, There<I>> for Stratum<E, Tail> {
    const INDEX: usize = Tail::INDEX;

    fn select(&self) -> &T {
        self.tail.select()
    }
}

/// Accepts a `T` for the input or router at the type-level index `I`.
#[diagnostic::on_unimplemented(
    message = "`{T}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait Feed<T, I, B> {
    /// Stores or routes `value`, marking every signal it changes.
    fn feed(&mut self, changed: &mut B, value: T);
}

impl<T: PartialEq, B: Flags, Tail: Len> Feed<T, Here, B> for Stratum<Input<T>, Tail> {
    fn feed(&mut self, changed: &mut B, value: T) {
        if self.entry.0 != value {
            self.entry.0 = value;
            changed.set(Tail::LEN);
        }
    }
}

impl<'a, In: ?Sized, F, B, Tail> Feed<&'a In, Here, B> for Stratum<Router<In, F>, Tail>
where
    F: FnMut(&In, &mut Sink<'_, Tail, B>),
{
    fn feed(&mut self, changed: &mut B, value: &'a In) {
        (self.entry.f)(
            value,
            &mut Sink {
                strata: &mut self.tail,
                changed,
            },
        );
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl<T: Absent, B> Feed<T, Nowhere, B> for Nil {
    fn feed(&mut self, _: &mut B, value: T) {
        value.absent();
    }
}

impl<T, I, B, E, Tail: Feed<T, I, B>> Feed<T, There<I>, B> for Stratum<E, Tail> {
    fn feed(&mut self, changed: &mut B, value: T) {
        self.tail.feed(changed, value);
    }
}

/// Recomputes every derived signal whose dependencies changed, bottom up.
pub trait Evaluate {
    /// Recomputes the dirty derived signals and marks the ones that changed.
    fn evaluate<B: Flags>(&mut self, changed: &mut B);
}

impl Evaluate for Nil {
    fn evaluate<B: Flags>(&mut self, _: &mut B) {}
}

impl<T, Tail: Evaluate> Evaluate for Stratum<Input<T>, Tail> {
    fn evaluate<B: Flags>(&mut self, changed: &mut B) {
        self.tail.evaluate(changed);
    }
}

impl<In: ?Sized, F, Tail: Evaluate> Evaluate for Stratum<Router<In, F>, Tail> {
    fn evaluate<B: Flags>(&mut self, changed: &mut B) {
        self.tail.evaluate(changed);
    }
}

impl<F, M, O, Tail> Evaluate for Stratum<Derived<F, M, O>, Tail>
where
    F: Compute<Tail, M, Output = O>,
    O: PartialEq,
    Tail: Evaluate + Len,
{
    fn evaluate<B: Flags>(&mut self, changed: &mut B) {
        self.tail.evaluate(changed);
        if F::dirty(changed) {
            let value = self.entry.f.compute(&self.tail);
            if value != self.entry.value {
                self.entry.value = value;
                changed.set(Tail::LEN);
            }
        }
    }
}

/// Reports every changed signal to a visitor `V`, bottom up.
pub trait Drain<V> {
    /// Visits the signals marked in `changed`.
    fn drain<B: Flags>(&self, changed: &B, visitor: &mut V);
}

impl<V> Drain<V> for Nil {
    fn drain<B: Flags>(&self, _: &B, _: &mut V) {}
}

impl<V, In: ?Sized, F, Tail: Drain<V>> Drain<V> for Stratum<Router<In, F>, Tail> {
    fn drain<B: Flags>(&self, changed: &B, visitor: &mut V) {
        self.tail.drain(changed, visitor);
    }
}

impl<V, E, Tail> Drain<V> for Stratum<E, Tail>
where
    E: Value,
    V: Visit<E::Signal>,
    Tail: Drain<V> + Len,
{
    fn drain<B: Flags>(&self, changed: &B, visitor: &mut V) {
        self.tail.drain(changed, visitor);
        if changed.get(Tail::LEN) {
            visitor.visit(self.entry.value());
        }
    }
}
