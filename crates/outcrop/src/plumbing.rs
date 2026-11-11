use core::borrow::Borrow;
use core::fmt::Debug;
use core::hash::Hash;

use crate::strata::Feed;
use crate::strata::Get;
use crate::strata::Strata;

pub trait Bits: Copy + Eq + Hash + Debug + Default {
    const BITS: usize;
    const ALL: Self;

    fn get(&self, index: usize) -> bool;

    fn set_top(&mut self, index: usize);

    fn clear_top(&mut self, index: usize);
}

pub trait Cursor {
    /// The bits left unused in the top byte of the flags.
    const SPARE: usize;
    type Next: Cursor;
    type Grow<F: Bits>: Bits;

    fn grow<F: Bits>(flags: F) -> Self::Grow<F>;

    fn below_mut<F: Bits>(flags: &mut Self::Grow<F>) -> &mut F;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not built by `Formation::input`, `memo`, `view` and `route`",
    note = "strata declared with `strata!` are complete and cannot be extended"
)]
pub trait LayeredImpl: Strata<Flags: Bits> {
    type Cursor: Cursor;
    /// The index of the next bit.
    const LEN: usize = <Self::Flags as Bits>::BITS - <Self::Cursor as Cursor>::SPARE;
}

/// One stratum, deposited on the strata `Tail`.
///
/// `Strata`, `LayeredImpl` and the forwarding of feeds and reports are
/// implemented once for every `(L, Tail)` from these items, whatever the
/// number of change bits the layer takes.
pub trait Layer<Tail: LayeredImpl>: Sized {
    /// The flags of `Tail` with this layer's bits added.
    type Flags: Bits;
    type Cursor: Cursor;
    /// Flags marking every signal as changed.
    const CHANGED: Self::Flags = <Self::Flags as Bits>::ALL;

    /// Adds this layer's bits to `flags`, marked as changed.
    fn deposit(flags: Tail::Flags) -> Self::Flags;

    fn below_mut(flags: &mut Self::Flags) -> &mut Tail::Flags;

    /// Updates this layer's bits, once `tail` is evaluated.
    fn evaluate(&mut self, tail: &Tail, changed: &mut Self::Flags);
}

/// A layer whose changed signals can be reported to the visitor `V`.
pub trait LayerReport<V, Tail: LayeredImpl>: Layer<Tail> {
    /// Reports the changed signals, clearing the bit of each before visiting
    /// it under `panic = "unwind"`.
    fn report(&self, tail: &Tail, changed: &mut Self::Flags, visitor: &mut V);
}

/// A layer holding a single signal. Not generic, unlike [`Settle`], so that
/// no other crate can implement it and the blanket [`Layer`] impl over it
/// stays apart from those of routers and banks.
pub trait Value {
    type Signal;
}

/// A layer holding a single signal, at the bit `Tail::LEN`.
pub trait Settle<Tail>: Value {
    type Ref<'a>: Borrow<Self::Signal>
    where
        Self: 'a,
        Tail: 'a;

    fn value<'a>(&'a self, tail: &'a Tail) -> Self::Ref<'a>;

    /// Updates the signal once `tail` is evaluated, returning whether it
    /// changed.
    fn settle<Fl: Bits>(&mut self, tail: &Tail, changed: &Fl) -> bool;
}

#[diagnostic::on_unimplemented(
    message = "`{T}` is not a signal declared above",
    label = "a signal can only depend on the inputs, memos and views declared above it"
)]
pub trait Select<T, I> {
    const INDEX: usize;
    type Ref<'a>: Borrow<T>
    where
        Self: 'a;

    fn select(&self) -> Self::Ref<'_>;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot derive a signal from the strata beneath it",
    label = "not a function of signals already in the formation",
    note = "every argument must be a reference `&T` to a distinct signal deposited earlier with `input`, `memo` or `view`, with at most eight arguments"
)]
pub trait ComputeImpl<S, M> {
    type Output;

    fn dirty<Fl: Bits>(changed: &Fl) -> bool;

    fn compute(&self, strata: &S) -> Self::Output;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; memos and views are computed by the formation and cannot be fed"
)]
pub trait Absent {}

pub trait Sound<I> {}

pub trait Distinct<T, I> {}

impl<S: Get<T, I>, T, I> Distinct<T, I> for S {}

pub trait Routed<T, I> {}

impl<S: Feed<T, I>, T, I> Routed<T, I> for S {}
