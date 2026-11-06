//! The building blocks a [`Formation`](crate::Formation) is made of.
//!
//! A formation's strata form a type-level list: every stratum wraps the ones
//! declared before it, ending in [`Nil`]. The types are opaque and only
//! appear in a formation's type. The traits are sealed bounds for generic
//! code over formations; they cannot be implemented outside this crate.

use core::fmt;
use core::marker::PhantomData;

use crate::capacity::Supported;
use crate::plumbing::Absent;
use crate::plumbing::CapacityImpl;
use crate::plumbing::ComputeImpl;
use crate::plumbing::FeedImpl;
use crate::plumbing::Flags;
use crate::plumbing::ReportImpl;
use crate::plumbing::Select;
use crate::plumbing::StrataImpl;
use crate::plumbing::Value;
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

/// A well-formed stack of strata.
pub trait Strata: StrataImpl {}

impl<S: StrataImpl> Strata for S {}

/// A function deriving a signal from the strata `S`.
///
/// Implemented for every function and closure taking one to eight references
/// to distinct signals, such as `fn(&A, &B) -> O`. The marker `M` records the
/// argument types and their positions so that the dependencies are inferred
/// from the signature.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot derive a signal from the strata beneath it",
    label = "not a function of signals already in the formation",
    note = "every argument must be a reference `&T` to a distinct signal deposited earlier with `input` or `with`, with at most eight arguments"
)]
pub trait Compute<S, M>: ComputeImpl<S, M> {}

impl<S, M, F: ComputeImpl<S, M>> Compute<S, M> for F {}

/// Strata accepting a `T` at the type-level index `I`, for a formation of
/// capacity `C`.
#[diagnostic::on_unimplemented(
    message = "`{T}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait Feed<T, I, C: Supported>: FeedImpl<T, I, C> {}

impl<S: FeedImpl<T, I, C>, T, I, C: Supported> Feed<T, I, C> for S {}

/// Strata whose every signal can be reported to the visitor `V`.
pub trait Report<V>: ReportImpl<V> {}

impl<S: ReportImpl<V>, V> Report<V> for S {}

impl StrataImpl for Nil {
    const LEN: usize = 0;

    fn evaluate<Fl: Flags>(&mut self, _: &mut Fl) {}
}

impl<T, Tail: StrataImpl> StrataImpl for Stratum<Input<T>, Tail> {
    const LEN: usize = Tail::LEN + 1;

    fn evaluate<Fl: Flags>(&mut self, changed: &mut Fl) {
        self.tail.evaluate(changed);
    }
}

impl<In: ?Sized, F, Tail: StrataImpl> StrataImpl for Stratum<Router<In, F>, Tail> {
    const LEN: usize = Tail::LEN;

    fn evaluate<Fl: Flags>(&mut self, changed: &mut Fl) {
        self.tail.evaluate(changed);
    }
}

impl<F, M, O, Tail> StrataImpl for Stratum<Derived<F, M, O>, Tail>
where
    F: ComputeImpl<Tail, M, Output = O>,
    O: PartialEq,
    Tail: StrataImpl,
{
    const LEN: usize = Tail::LEN + 1;

    fn evaluate<Fl: Flags>(&mut self, changed: &mut Fl) {
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

impl<E: Value, Tail: StrataImpl> Select<E::Signal, Here> for Stratum<E, Tail> {
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

impl<T: PartialEq, C: Supported, Tail: StrataImpl> FeedImpl<T, Here, C>
    for Stratum<Input<T>, Tail>
{
    fn feed(&mut self, changed: &mut <C as CapacityImpl>::Flags, value: T) {
        if self.entry.0 != value {
            self.entry.0 = value;
            changed.set(Tail::LEN);
        }
    }
}

impl<'a, In: ?Sized, F, C: Supported, Tail> FeedImpl<&'a In, Here, C>
    for Stratum<Router<In, F>, Tail>
where
    F: FnMut(&In, &mut Sink<'_, Tail, C>),
{
    fn feed(&mut self, changed: &mut <C as CapacityImpl>::Flags, value: &'a In) {
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
impl<T: Absent, C: Supported> FeedImpl<T, Nowhere, C> for Nil {
    fn feed(&mut self, _: &mut <C as CapacityImpl>::Flags, value: T) {
        value.absent();
    }
}

impl<T, I, C: Supported, E, Tail: FeedImpl<T, I, C>> FeedImpl<T, There<I>, C> for Stratum<E, Tail> {
    fn feed(&mut self, changed: &mut <C as CapacityImpl>::Flags, value: T) {
        self.tail.feed(changed, value);
    }
}

impl<V> ReportImpl<V> for Nil {
    fn report<Fl: Flags>(&self, _: &Fl, _: &mut V) {}
}

impl<V, In: ?Sized, F, Tail: ReportImpl<V>> ReportImpl<V> for Stratum<Router<In, F>, Tail> {
    fn report<Fl: Flags>(&self, changed: &Fl, visitor: &mut V) {
        self.tail.report(changed, visitor);
    }
}

impl<V, E, Tail> ReportImpl<V> for Stratum<E, Tail>
where
    E: Value,
    V: Visit<E::Signal>,
    Tail: ReportImpl<V> + StrataImpl,
{
    fn report<Fl: Flags>(&self, changed: &Fl, visitor: &mut V) {
        self.tail.report(changed, visitor);
        if changed.get(Tail::LEN) {
            visitor.visit(self.entry.value());
        }
    }
}
