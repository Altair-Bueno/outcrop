use core::fmt;
use core::marker::PhantomData;

use crate::flags::Byte;
use crate::flags::Used;
use crate::plumbing::Absent;
use crate::plumbing::ComputeImpl;
use crate::plumbing::Cursor;
use crate::plumbing::FeedImpl;
use crate::plumbing::Flags as _;
use crate::plumbing::ReportImpl;
use crate::plumbing::Select;
use crate::plumbing::Stack;
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

/// The strata a [`Formation`](crate::Formation) is made of.
///
/// Generic code over a formation bounds its strata with `S: Strata`. This
/// trait and the other formation bounds are sealed.
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

/// Strata accepting a `T` at the type-level index `I`.
#[diagnostic::on_unimplemented(
    message = "`{T}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait Feed<T, I>: FeedImpl<T, I> {}

impl<S: FeedImpl<T, I>, T, I> Feed<T, I> for S {}

/// Strata whose every signal can be reported to the visitor `V`.
pub trait Report<V>: ReportImpl<V> {}

impl<S: ReportImpl<V>, V> Report<V> for S {}

type Grown<Tail> = <<Tail as StrataImpl>::Cursor as Cursor>::Grow<<Tail as StrataImpl>::Flags>;

type Next<Tail> = <<Tail as StrataImpl>::Cursor as Cursor>::Next;

impl StrataImpl for Nil {
    const LEN: usize = 0;
    type Flags = Byte;
    type Cursor = Used<0>;

    fn evaluate(&mut self, _: &mut Byte) {}
}

impl<T, Tail: StrataImpl> StrataImpl for Stratum<Input<T>, Tail> {
    const LEN: usize = Tail::LEN + 1;
    type Flags = Grown<Tail>;
    type Cursor = Next<Tail>;

    fn evaluate(&mut self, changed: &mut Self::Flags) {
        self.tail.evaluate(Self::below_mut(changed));
    }
}

impl<F, M, O, Tail> StrataImpl for Stratum<Derived<F, M, O>, Tail>
where
    F: ComputeImpl<Tail, M, Output = O>,
    O: PartialEq,
    Tail: StrataImpl,
{
    const LEN: usize = Tail::LEN + 1;
    type Flags = Grown<Tail>;
    type Cursor = Next<Tail>;

    fn evaluate(&mut self, changed: &mut Self::Flags) {
        self.tail.evaluate(Self::below_mut(changed));
        if F::dirty(changed) {
            let value = self.entry.f.compute(&self.tail);
            if value != self.entry.value {
                self.entry.value = value;
                changed.set_top(Tail::LEN);
            }
        }
    }
}

impl<In: ?Sized, F, Tail: StrataImpl> StrataImpl for Stratum<Router<In, F>, Tail> {
    const LEN: usize = Tail::LEN;
    type Flags = Tail::Flags;
    type Cursor = Tail::Cursor;

    fn evaluate(&mut self, changed: &mut Self::Flags) {
        self.tail.evaluate(changed);
    }
}

impl<E: Value, Tail: StrataImpl> Stack for Stratum<E, Tail>
where
    Self: StrataImpl<Flags = Grown<Tail>>,
{
    type Tail = Tail;

    fn below(flags: &Self::Flags) -> &Tail::Flags {
        Tail::Cursor::below(flags)
    }

    fn below_mut(flags: &mut Self::Flags) -> &mut Tail::Flags {
        Tail::Cursor::below_mut(flags)
    }
}

impl<In: ?Sized, F, Tail: StrataImpl> Stack for Stratum<Router<In, F>, Tail> {
    type Tail = Tail;

    fn below(flags: &Self::Flags) -> &Tail::Flags {
        flags
    }

    fn below_mut(flags: &mut Self::Flags) -> &mut Tail::Flags {
        flags
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

impl<T: PartialEq, Tail: StrataImpl> FeedImpl<T, Here> for Stratum<Input<T>, Tail> {
    fn feed(&mut self, changed: &mut Self::Flags, value: T) {
        if self.entry.0 != value {
            self.entry.0 = value;
            changed.set_top(Tail::LEN);
        }
    }
}

impl<'a, In: ?Sized, F, Tail> FeedImpl<&'a In, Here> for Stratum<Router<In, F>, Tail>
where
    F: FnMut(&In, &mut Sink<'_, Tail>),
    Tail: StrataImpl,
{
    fn feed(&mut self, changed: &mut Self::Flags, value: &'a In) {
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
impl<T: Absent> FeedImpl<T, Nowhere> for Nil {
    fn feed(&mut self, _: &mut Byte, _: T) {}
}

impl<T, I, E, Tail> FeedImpl<T, There<I>> for Stratum<E, Tail>
where
    Self: Stack<Tail = Tail>,
    Tail: FeedImpl<T, I>,
{
    fn feed(&mut self, changed: &mut Self::Flags, value: T) {
        self.tail.feed(Self::below_mut(changed), value);
    }
}

impl<V> ReportImpl<V> for Nil {
    fn report(&self, _: &Byte, _: &mut V) {}
}

impl<V, In: ?Sized, F, Tail: ReportImpl<V>> ReportImpl<V> for Stratum<Router<In, F>, Tail> {
    fn report(&self, changed: &Self::Flags, visitor: &mut V) {
        self.tail.report(Self::below(changed), visitor);
    }
}

impl<V, E, Tail> ReportImpl<V> for Stratum<E, Tail>
where
    Self: Stack<Tail = Tail>,
    E: Value,
    V: Visit<E::Signal>,
    Tail: ReportImpl<V>,
{
    fn report(&self, changed: &Self::Flags, visitor: &mut V) {
        self.tail.report(Self::below(changed), visitor);
        if changed.get(Tail::LEN) {
            visitor.visit(self.entry.value());
        }
    }
}
