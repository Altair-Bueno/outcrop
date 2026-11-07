use core::fmt::Debug;

pub trait Flags: Debug + Default {
    const BYTES: usize;

    fn get(&self, index: usize) -> bool;

    fn set_top(&mut self, index: usize);
}

pub trait Cursor {
    type Next: Cursor;
    type Grow<F: Flags>: Flags;

    fn grow<F: Flags>(flags: F) -> Self::Grow<F>;

    fn below<F: Flags>(flags: &Self::Grow<F>) -> &F;

    fn below_mut<F: Flags>(flags: &mut Self::Grow<F>) -> &mut F;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a stack of strata",
    note = "strata are built with `Formation::input`, `with` and `route`; `Strata` is sealed"
)]
pub trait StrataImpl {
    const LEN: usize;
    type Flags: Flags;
    type Cursor: Cursor;

    fn evaluate(&mut self, changed: &mut Self::Flags);
}

pub trait Stack: StrataImpl {
    type Tail: StrataImpl;

    fn below(flags: &Self::Flags) -> &<Self::Tail as StrataImpl>::Flags;

    fn below_mut(flags: &mut Self::Flags) -> &mut <Self::Tail as StrataImpl>::Flags;
}

pub trait Value {
    type Signal;

    fn value(&self) -> &Self::Signal;
}

pub trait Select<T, I> {
    const INDEX: usize;

    fn select(&self) -> &T;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot derive a signal from the strata beneath it",
    label = "not a function of signals already in the formation",
    note = "every argument must be a reference `&T` to a distinct signal deposited earlier with `input` or `with`, with at most eight arguments"
)]
pub trait ComputeImpl<S, M> {
    type Output;

    fn dirty<Fl: Flags>(changed: &Fl) -> bool;

    fn compute(&mut self, strata: &S) -> Self::Output;
}

#[diagnostic::on_unimplemented(
    message = "`{T}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait FeedImpl<T, I>: StrataImpl {
    fn feed(&mut self, changed: &mut Self::Flags, value: T);
}

pub trait ReportImpl<V>: StrataImpl {
    fn report(&self, changed: &Self::Flags, visitor: &mut V);
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait Absent {
    fn absent(self);
}
