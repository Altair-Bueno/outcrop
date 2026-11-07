use core::fmt;
use core::marker::PhantomData;

use crate::flags::Byte;
use crate::flags::Used;
use crate::plumbing::Absent;
use crate::plumbing::Bits;
use crate::plumbing::ComputeImpl;
use crate::plumbing::Cursor;
use crate::plumbing::LayeredImpl;
use crate::plumbing::Select;
use crate::plumbing::Stack;
use crate::plumbing::Value;
use crate::sink::Sink;
use crate::strata::Feed;
use crate::strata::Report;
use crate::strata::Strata;
use crate::visit::Visit;

/// The empty bottom of every formation.
#[derive(Debug, Clone, Copy)]
pub struct Nil;

/// One stratum `E` deposited on top of the strata `Tail`.
#[derive(Debug, Clone, Copy)]
pub struct Stratum<E, Tail> {
    pub(super) entry: E,
    pub(super) tail: Tail,
}

/// An input signal fed by the caller.
#[derive(Debug, Clone, Copy)]
pub struct Input<T>(pub(super) T);

/// A signal derived from the strata beneath it by `F`.
#[derive(Clone, Copy)]
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

impl<In: ?Sized, F: Clone> Clone for Router<In, F> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            marker: PhantomData,
        }
    }
}

impl<In: ?Sized, F: Copy> Copy for Router<In, F> {}

/// Type-level index of the topmost stratum.
#[derive(Debug, Clone, Copy)]
pub struct Here;

/// Type-level index of a stratum beneath the topmost one.
#[derive(Debug, Clone, Copy)]
pub struct There<I>(PhantomData<I>);

/// Type-level index past the bottom of the strata, where nothing is found.
#[derive(Debug, Clone, Copy)]
pub struct Nowhere;

/// Strata built with [`Formation::input`](crate::Formation::input), `with`
/// and `route`, which can be extended further.
///
/// This trait is sealed.
pub trait Layered: LayeredImpl {}

impl<S: LayeredImpl> Layered for S {}

/// A function deriving a signal from the strata `S`.
///
/// Implemented for every function and closure taking one to eight references
/// to distinct signals, such as `fn(&A, &B) -> O`. The marker `M` records the
/// argument types and their positions so that the dependencies are inferred
/// from the signature. This trait is sealed.
pub trait Compute<S, M>: ComputeImpl<S, M> {}

impl<S, M, F: ComputeImpl<S, M>> Compute<S, M> for F {}

type Grown<Tail> = <<Tail as LayeredImpl>::Cursor as Cursor>::Grow<<Tail as Strata>::Flags>;

type Next<Tail> = <<Tail as LayeredImpl>::Cursor as Cursor>::Next;

impl Strata for Nil {
    type Flags = Byte;

    const CHANGED: Byte = Byte::EMPTY;

    fn evaluate(&mut self, _: &mut Byte) {}
}

impl LayeredImpl for Nil {
    const LEN: usize = 0;
    type Cursor = Used<0>;
}

impl<T, Tail: LayeredImpl> Strata for Stratum<Input<T>, Tail> {
    type Flags = Grown<Tail>;

    const CHANGED: Self::Flags = <Self::Flags as Bits>::ALL;

    fn evaluate(&mut self, changed: &mut Self::Flags) {
        self.tail.evaluate(Self::below_mut(changed));
    }
}

impl<T, Tail: LayeredImpl> LayeredImpl for Stratum<Input<T>, Tail> {
    const LEN: usize = Tail::LEN + 1;
    type Cursor = Next<Tail>;
}

impl<F, M, O, Tail> Strata for Stratum<Derived<F, M, O>, Tail>
where
    F: ComputeImpl<Tail, M, Output = O>,
    O: PartialEq,
    Tail: LayeredImpl,
{
    type Flags = Grown<Tail>;

    const CHANGED: Self::Flags = <Self::Flags as Bits>::ALL;

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

impl<F, M, O, Tail> LayeredImpl for Stratum<Derived<F, M, O>, Tail>
where
    F: ComputeImpl<Tail, M, Output = O>,
    O: PartialEq,
    Tail: LayeredImpl,
{
    const LEN: usize = Tail::LEN + 1;
    type Cursor = Next<Tail>;
}

impl<In: ?Sized, F, Tail: LayeredImpl> Strata for Stratum<Router<In, F>, Tail> {
    type Flags = Tail::Flags;

    const CHANGED: Self::Flags = Tail::CHANGED;

    fn evaluate(&mut self, changed: &mut Self::Flags) {
        self.tail.evaluate(changed);
    }
}

impl<In: ?Sized, F, Tail: LayeredImpl> LayeredImpl for Stratum<Router<In, F>, Tail> {
    const LEN: usize = Tail::LEN;
    type Cursor = Tail::Cursor;
}

impl<E: Value, Tail: LayeredImpl> Stack for Stratum<E, Tail>
where
    Self: LayeredImpl<Flags = Grown<Tail>>,
{
    type Tail = Tail;

    fn below(flags: &Self::Flags) -> &Tail::Flags {
        Tail::Cursor::below(flags)
    }

    fn below_mut(flags: &mut Self::Flags) -> &mut Tail::Flags {
        Tail::Cursor::below_mut(flags)
    }
}

impl<In: ?Sized, F, Tail: LayeredImpl> Stack for Stratum<Router<In, F>, Tail> {
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

impl<E: Value, Tail: LayeredImpl> Select<E::Signal, Here> for Stratum<E, Tail> {
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

impl<T: PartialEq, Tail: LayeredImpl> Feed<T, Here> for Stratum<Input<T>, Tail> {
    fn feed(&mut self, changed: &mut Self::Flags, value: T) {
        if self.entry.0 != value {
            self.entry.0 = value;
            changed.set_top(Tail::LEN);
        }
    }
}

impl<'a, In: ?Sized, F, Tail> Feed<&'a In, Here> for Stratum<Router<In, F>, Tail>
where
    F: FnMut(&In, &mut Sink<'_, Tail>),
    Tail: LayeredImpl,
{
    fn feed(&mut self, changed: &mut Self::Flags, value: &'a In) {
        (self.entry.f)(value, &mut Sink::new(&mut self.tail, changed));
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl<T: Absent> Feed<T, Nowhere> for Nil {
    fn feed(&mut self, _: &mut Byte, _: T) {}
}

impl<T, I, E, Tail> Feed<T, There<I>> for Stratum<E, Tail>
where
    Self: Stack<Tail = Tail>,
    Tail: Feed<T, I>,
{
    fn feed(&mut self, changed: &mut Self::Flags, value: T) {
        self.tail.feed(Self::below_mut(changed), value);
    }
}

impl<V> Report<V> for Nil {
    fn report(&self, _: &Byte, _: &mut V) {}
}

impl<V, In: ?Sized, F, Tail> Report<V> for Stratum<Router<In, F>, Tail>
where
    Tail: LayeredImpl + Report<V>,
{
    fn report(&self, changed: &Self::Flags, visitor: &mut V) {
        self.tail.report(Self::below(changed), visitor);
    }
}

impl<V, E, Tail> Report<V> for Stratum<E, Tail>
where
    Self: Stack<Tail = Tail>,
    E: Value,
    V: Visit<E::Signal>,
    Tail: LayeredImpl + Report<V>,
{
    fn report(&self, changed: &Self::Flags, visitor: &mut V) {
        self.tail.report(Self::below(changed), visitor);
        if changed.get(Tail::LEN) {
            visitor.visit(self.entry.value());
        }
    }
}
