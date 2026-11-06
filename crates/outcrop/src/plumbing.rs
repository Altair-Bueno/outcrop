use core::fmt::Debug;

pub trait Flags {
    fn get(&self, index: usize) -> bool;

    fn set(&mut self, index: usize);
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a formation capacity",
    note = "formation capacities are `Capacity<N>` with `N` between 1 and 1024; `Supported` is sealed"
)]
pub trait CapacityImpl {
    type Flags: Flags + Default + Debug;
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a stack of strata",
    note = "strata are built with `Formation::input`, `with` and `route`; `Strata` is sealed"
)]
pub trait StrataImpl {
    const LEN: usize;

    fn evaluate<Fl: Flags>(&mut self, changed: &mut Fl);
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
pub trait FeedImpl<T, I, C: CapacityImpl> {
    fn feed(&mut self, changed: &mut C::Flags, value: T);
}

pub trait ReportImpl<V> {
    fn report<Fl: Flags>(&self, changed: &Fl, visitor: &mut V);
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be fed to this formation",
    label = "not an input or router of this formation",
    note = "inputs are fed by value and routers by reference; derived signals are computed by the formation and cannot be fed"
)]
pub trait Absent {
    fn absent(self);
}
