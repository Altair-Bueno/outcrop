use core::fmt::Debug;
use core::hash::Hash;

use crate::strata::Strata;

pub trait Bits: Copy + Eq + Hash + Debug + Default {
    const BYTES: usize;
    const ALL: Self;

    fn get(&self, index: usize) -> bool;

    fn set_top(&mut self, index: usize);
}

pub trait Cursor {
    type Next: Cursor;
    type Grow<F: Bits>: Bits;

    fn grow<F: Bits>(flags: F) -> Self::Grow<F>;

    fn below<F: Bits>(flags: &Self::Grow<F>) -> &F;

    fn below_mut<F: Bits>(flags: &mut Self::Grow<F>) -> &mut F;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not built by `Formation::input`, `with` and `route`",
    note = "strata declared with `strata!` are complete and cannot be extended"
)]
pub trait LayeredImpl: Strata<Flags: Bits> {
    const LEN: usize;
    type Cursor: Cursor;
}

pub trait Stack: LayeredImpl {
    type Tail: LayeredImpl;

    fn below(flags: &Self::Flags) -> &<Self::Tail as Strata>::Flags;

    fn below_mut(flags: &mut Self::Flags) -> &mut <Self::Tail as Strata>::Flags;
}

pub trait Value {
    type Signal;

    fn value(&self) -> &Self::Signal;
}

pub trait Settle<Tail> {
    fn stale<Fl: Bits>(changed: &Fl) -> bool;
}

pub trait Select<T, I> {
    const INDEX: usize;

    fn select(&self) -> &T;

    fn stale<Fl: Bits>(changed: &Fl) -> bool;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot derive a signal from the strata beneath it",
    label = "not a function of signals already in the formation",
    note = "every argument must be a reference `&T` to a distinct signal deposited earlier with `input` or `with`, with at most eight arguments"
)]
pub trait ComputeImpl<S, M> {
    type Output;

    fn dirty<Fl: Bits>(changed: &Fl) -> bool;

    fn stale<Fl: Bits>(changed: &Fl) -> bool;

    fn compute(&mut self, strata: &S) -> Self::Output;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait Absent {}
