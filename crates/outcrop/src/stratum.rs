use core::borrow::Borrow as _;
use core::convert::Infallible;
use core::fmt;
use core::marker::PhantomData;

use crate::bank::Element;
use crate::bank::Slot;
use crate::bank::Write;
use crate::flags::Chunk;
use crate::flags::Used;
use crate::plumbing::Absent;
use crate::plumbing::Bits;
use crate::plumbing::ComputeImpl;
use crate::plumbing::Cursor;
use crate::plumbing::Layer;
use crate::plumbing::LayerReport;
use crate::plumbing::LayeredImpl;
use crate::plumbing::Select;
use crate::plumbing::Settle;
use crate::plumbing::Value;
use crate::reading::Reading;
use crate::sink::Sink;
use crate::strata::Feed;
use crate::strata::Get;
use crate::strata::Report;
use crate::strata::Strata;
use crate::visit::Visit;

/// An input signal fed by the caller.
#[derive(Debug, Clone, Copy)]
pub struct Input<T>(pub(super) T);

/// An array of inputs fed by the caller, with a change bit per element,
/// packed in `BYTES` bytes.
#[derive(Debug, Clone, Copy)]
pub struct Bank<T, const N: usize, const BYTES: usize>(pub(super) [T; N]);

/// A signal computed by `F` from the strata beneath it, and cached.
#[derive(Clone, Copy)]
pub struct Memo<F, M, O> {
    pub(super) f: F,
    pub(super) value: O,
    pub(super) marker: PhantomData<fn() -> M>,
}

impl<F, M, O: fmt::Debug> fmt::Debug for Memo<F, M, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Memo")
            .field("value", &self.value)
            .finish_non_exhaustive()
    }
}

/// A signal computed by `F` from the strata beneath it whenever it is read.
pub struct View<F, M, O> {
    pub(super) f: F,
    pub(super) marker: PhantomData<fn() -> (M, O)>,
}

impl<F, M, O> fmt::Debug for View<F, M, O> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("View").finish_non_exhaustive()
    }
}

impl<F: Clone, M, O> Clone for View<F, M, O> {
    fn clone(&self) -> Self {
        Self {
            f: self.f.clone(),
            marker: PhantomData,
        }
    }
}

impl<F: Copy, M, O> Copy for View<F, M, O> {}

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

/// Strata built with [`Formation::input`](crate::Formation::input), `memo`,
/// `view` and `route`, which can be extended further.
///
/// This trait is sealed.
pub trait Layered: LayeredImpl {}

impl<S: LayeredImpl> Layered for S {}

/// A function computing a signal from the strata `S`.
///
/// Implemented for every function and closure taking one to eight references
/// to distinct signals, such as `fn(&A, &B) -> O`. The marker `M` records the
/// argument types and their positions so that the dependencies are inferred
/// from the signature. This trait is sealed.
pub trait Compute<S, M>: ComputeImpl<S, M> {}

impl<S, M, F: ComputeImpl<S, M>> Compute<S, M> for F {}

type Grown<Tail> = <<Tail as LayeredImpl>::Cursor as Cursor>::Grow<<Tail as Strata>::Flags>;

type Next<Tail> = <<Tail as LayeredImpl>::Cursor as Cursor>::Next;

impl Strata for () {
    type Flags = u8;

    const CHANGED: u8 = 0;

    fn evaluate(&mut self, _: &mut u8) {}
}

impl LayeredImpl for () {
    const LEN: usize = 0;
    type Cursor = Used<0>;
}

impl<L: Layer<Tail>, Tail: LayeredImpl> Strata for (L, Tail) {
    type Flags = L::Flags;

    const CHANGED: Self::Flags = L::CHANGED;

    fn evaluate(&mut self, changed: &mut Self::Flags) {
        self.1.evaluate(L::below_mut(changed));
        self.0.evaluate(&self.1, changed);
    }
}

impl<L: Layer<Tail>, Tail: LayeredImpl> LayeredImpl for (L, Tail) {
    const LEN: usize = L::LEN;
    type Cursor = L::Cursor;
}

impl<E: Value + Settle<Tail>, Tail: LayeredImpl> Layer<Tail> for E {
    type Flags = Grown<Tail>;
    const LEN: usize = Tail::LEN + 1;
    type Cursor = Next<Tail>;

    fn deposit(flags: Tail::Flags) -> Grown<Tail> {
        let mut flags = Tail::Cursor::grow(flags);
        flags.set_top(Tail::LEN);
        flags
    }

    fn below(flags: &Grown<Tail>) -> &Tail::Flags {
        Tail::Cursor::below(flags)
    }

    fn below_mut(flags: &mut Grown<Tail>) -> &mut Tail::Flags {
        Tail::Cursor::below_mut(flags)
    }

    fn evaluate(&mut self, tail: &Tail, changed: &mut Grown<Tail>) {
        self.settle(tail, changed, Tail::LEN);
    }
}

impl<V, E, Tail> LayerReport<V, Tail> for E
where
    E: Value + Settle<Tail>,
    V: Visit<E::Signal>,
    Tail: LayeredImpl,
{
    fn report(&self, tail: &Tail, changed: &Grown<Tail>, visitor: &mut V) {
        if changed.get(Tail::LEN) {
            visitor.visit(self.value(tail).borrow());
        }
    }
}

impl<In: ?Sized, F, Tail: LayeredImpl> Layer<Tail> for Router<In, F> {
    type Flags = Tail::Flags;
    const LEN: usize = Tail::LEN;
    type Cursor = Tail::Cursor;
    const CHANGED: Tail::Flags = Tail::CHANGED;

    fn deposit(flags: Tail::Flags) -> Tail::Flags {
        flags
    }

    fn below(flags: &Tail::Flags) -> &Tail::Flags {
        flags
    }

    fn below_mut(flags: &mut Tail::Flags) -> &mut Tail::Flags {
        flags
    }

    fn evaluate(&mut self, _: &Tail, _: &mut Tail::Flags) {}
}

impl<V, In: ?Sized, F, Tail: LayeredImpl> LayerReport<V, Tail> for Router<In, F> {
    fn report(&self, _: &Tail, _: &Tail::Flags, _: &mut V) {}
}

type Banked<Tail, const BYTES: usize> = (<Tail as Strata>::Flags, Chunk<BYTES>);

impl<T, const N: usize, const BYTES: usize, Tail> Layer<Tail> for Bank<T, N, BYTES>
where
    Tail: LayeredImpl,
{
    type Flags = Banked<Tail, BYTES>;
    const LEN: usize = <Self::Flags as Bits>::BITS;
    type Cursor = Used<8>;

    fn deposit(flags: Tail::Flags) -> Self::Flags {
        (flags, Chunk::ALL)
    }

    fn below(flags: &Self::Flags) -> &Tail::Flags {
        &flags.0
    }

    fn below_mut(flags: &mut Self::Flags) -> &mut Tail::Flags {
        &mut flags.0
    }

    fn evaluate(&mut self, _: &Tail, _: &mut Self::Flags) {}
}

impl<V, T, const N: usize, const BYTES: usize, Tail> LayerReport<V, Tail> for Bank<T, N, BYTES>
where
    V: for<'a> Visit<Element<'a, T>>,
    Tail: LayeredImpl,
{
    fn report(&self, _: &Tail, changed: &Self::Flags, visitor: &mut V) {
        Element::report(&self.0, |index| changed.1.get(index), visitor);
    }
}

impl<T> Value for Input<T> {
    type Signal = T;
}

impl<F, M, O> Value for Memo<F, M, O> {
    type Signal = O;
}

impl<F, M, O> Value for View<F, M, O> {
    type Signal = O;
}

impl<T, Tail> Settle<Tail> for Input<T> {
    type Ref<'a>
        = &'a T
    where
        Self: 'a,
        Tail: 'a;

    fn value<'a>(&'a self, _: &'a Tail) -> &'a T {
        &self.0
    }

    fn stale<Fl: Bits>(_: &Fl) -> bool {
        false
    }

    fn moved<Fl: Bits>(_: &Fl) -> bool {
        false
    }

    fn settle<Fl: Bits>(&mut self, _: &Tail, _: &mut Fl, _: usize) {}
}

impl<F, M, O, Tail> Settle<Tail> for Memo<F, M, O>
where
    F: ComputeImpl<Tail, M, Output = O>,
    O: PartialEq,
{
    type Ref<'a>
        = &'a O
    where
        Self: 'a,
        Tail: 'a;

    fn value<'a>(&'a self, _: &'a Tail) -> &'a O {
        &self.value
    }

    fn stale<Fl: Bits>(changed: &Fl) -> bool {
        F::moved(changed)
    }

    fn moved<Fl: Bits>(changed: &Fl) -> bool {
        F::moved(changed)
    }

    fn settle<Fl: Bits>(&mut self, tail: &Tail, changed: &mut Fl, index: usize) {
        if F::dirty(changed) {
            let value = self.f.compute(tail);
            if value != self.value {
                self.value = value;
                changed.set_top(index);
            }
        }
    }
}

impl<F, M, O, Tail> Settle<Tail> for View<F, M, O>
where
    F: ComputeImpl<Tail, M, Output = O>,
{
    type Ref<'a>
        = O
    where
        Self: 'a,
        Tail: 'a;

    fn value<'a>(&'a self, tail: &'a Tail) -> O {
        self.f.compute(tail)
    }

    fn stale<Fl: Bits>(changed: &Fl) -> bool {
        F::pending(changed)
    }

    fn moved<Fl: Bits>(changed: &Fl) -> bool {
        F::moved(changed)
    }

    fn settle<Fl: Bits>(&mut self, _: &Tail, changed: &mut Fl, index: usize) {
        if F::dirty(changed) {
            changed.set_top(index);
        }
    }
}

impl<E: Settle<Tail>, Tail: LayeredImpl> Select<E::Signal, ()> for (E, Tail) {
    const INDEX: usize = Tail::LEN;
    type Ref<'a>
        = E::Ref<'a>
    where
        Self: 'a;

    fn select(&self) -> E::Ref<'_> {
        self.0.value(&self.1)
    }

    fn stale<Fl: Bits>(changed: &Fl) -> bool {
        E::stale(changed)
    }

    fn moved<Fl: Bits>(changed: &Fl) -> bool {
        changed.get(Tail::LEN) || E::moved(changed)
    }
}

/// Index of a slot in [`Select`] and [`Feed`], telling it apart from the
/// signals deposited one by one.
#[derive(Debug)]
pub enum Slotted {}

impl<E, const N: usize, const BYTES: usize, Tail> Select<E, Slotted>
    for (Bank<E::Item, N, BYTES>, Tail)
where
    E: Slot,
    Tail: LayeredImpl,
{
    const INDEX: usize = <Tail::Flags as Bits>::BITS + E::INDEX;
    type Ref<'a>
        = E
    where
        Self: 'a;

    fn select(&self) -> E {
        read(&self.0.0)
    }

    fn stale<Fl: Bits>(_: &Fl) -> bool {
        false
    }

    fn moved<Fl: Bits>(changed: &Fl) -> bool {
        changed.get(<Self as Select<E, Slotted>>::INDEX)
    }
}

impl<T, I, E, Tail: Select<T, I>> Select<T, (I,)> for (E, Tail) {
    const INDEX: usize = Tail::INDEX;
    type Ref<'a>
        = Tail::Ref<'a>
    where
        Self: 'a;

    fn select(&self) -> Tail::Ref<'_> {
        self.1.select()
    }

    fn stale<Fl: Bits>(changed: &Fl) -> bool {
        Tail::stale(changed)
    }

    fn moved<Fl: Bits>(changed: &Fl) -> bool {
        Tail::moved(changed)
    }
}

impl<T, I, S: LayeredImpl + Select<T, I>> Get<T, I> for S {
    type Ref<'a>
        = <S as Select<T, I>>::Ref<'a>
    where
        Self: 'a;

    fn get(&self, changed: &Self::Flags) -> Reading<Self::Ref<'_>> {
        let value = self.select();
        if Self::stale(changed) {
            Reading::Stale(value)
        } else {
            Reading::Fresh(value)
        }
    }
}

impl<T: PartialEq, Tail: LayeredImpl> Feed<T, ()> for (Input<T>, Tail) {
    fn feed(&mut self, changed: &mut Self::Flags, value: T) {
        if self.0.0 != value {
            self.0.0 = value;
            changed.set_top(Tail::LEN);
        }
    }
}

impl<'a, In: ?Sized, F, Tail> Feed<&'a In, ()> for (Router<In, F>, Tail)
where
    F: FnMut(&In, &mut Sink<'_, Tail>),
    Tail: LayeredImpl,
{
    fn feed(&mut self, changed: &mut Self::Flags, value: &'a In) {
        (self.0.f)(value, &mut Sink::new(&mut self.1, changed));
    }
}

impl<'w, T, const N: usize, const BYTES: usize, Tail> Feed<Write<'w, T>, Slotted>
    for (Bank<T, N, BYTES>, Tail)
where
    T: PartialEq + Clone,
    Tail: LayeredImpl,
{
    fn feed(&mut self, changed: &mut Self::Flags, write: Write<'w, T>) {
        write.apply(&mut self.0.0, |index| changed.1.set(index));
    }
}

#[cfg_attr(coverage_nightly, coverage(off))]
impl<T: Absent> Feed<T, Infallible> for () {
    fn feed(&mut self, _: &mut u8, _: T) {}
}

impl<T, I, L, Tail> Feed<T, (I,)> for (L, Tail)
where
    L: Layer<Tail>,
    Tail: LayeredImpl + Feed<T, I>,
{
    fn feed(&mut self, changed: &mut Self::Flags, value: T) {
        self.1.feed(L::below_mut(changed), value);
    }
}

impl<V> Report<V> for () {
    fn report(&self, _: &u8, _: &mut V) {}
}

impl<V, L, Tail> Report<V> for (L, Tail)
where
    L: LayerReport<V, Tail>,
    Tail: LayeredImpl + Report<V>,
{
    fn report(&self, changed: &Self::Flags, visitor: &mut V) {
        self.1.report(L::below(changed), visitor);
        self.0.report(&self.1, changed, visitor);
    }
}

/// Reads the slot `E` from `bank`, rejecting at compile time a slot past its
/// end.
fn read<E: Slot, const N: usize>(bank: &[E::Item; N]) -> E {
    const { assert!(E::INDEX < N, "the slot is past the end of the bank") };
    E::read(&bank[E::INDEX])
}
