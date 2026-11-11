//! Support for the code `strata!` generates. Not public API.

use core::borrow::Borrow;
use core::marker::PhantomData;

use crate::bank::Element;
use crate::bank::Write;
pub use crate::flags::Chunk;
use crate::nested::Nested;
use crate::plumbing::ComputeImpl;
use crate::plumbing::Distinct;
use crate::plumbing::Routed;
use crate::plumbing::Select;
use crate::reading::Reading;
use crate::strata::Feed;
use crate::strata::Strata;
use crate::stratum::Bank;
use crate::stratum::Input;
use crate::stratum::View;
pub use crate::stratum::read;
pub use crate::stratum::slot;
use crate::visit::Visit;

/// A signal `T` of a `strata!` struct, found by type. `BIT` is the signal's
/// bit in the struct's `Chunk` flags. `CACHED` tells whether the signal is a
/// memo or computed from one, so that reading it while any bit is set is
/// [`Stale`](Reading::Stale).
pub trait Signal<T, I>: Strata {
    const BIT: usize;
    const CACHED: bool;
    type Ref<'a>: Borrow<T>
    where
        Self: 'a;

    fn value(&self) -> Self::Ref<'_>;
}

pub const fn bit<S: Signal<T, I>, T, I>() -> usize {
    S::BIT
}

pub const fn cached<S: Signal<T, I>, T, I>() -> bool {
    S::CACHED
}

pub fn value<S: Signal<T, I>, T, I>(strata: &S) -> S::Ref<'_> {
    strata.value()
}

pub fn get<'s, S, T, I, const N: usize>(strata: &'s S, changed: &Chunk<N>) -> Reading<S::Ref<'s>>
where
    S: Signal<T, I, Flags = Chunk<N>>,
{
    Reading::new(strata.value(), S::CACHED && *changed != Chunk::default())
}

pub fn routed<S: Feed<T, I>, T, I>() {}

pub fn ne<T: Eq>(a: &T, b: &T) -> bool {
    a != b
}

pub const fn all<const N: usize>() -> Chunk<N> {
    Chunk::ALL
}

pub fn is_set<const N: usize>(changed: &Chunk<N>, bit: usize) -> bool {
    changed.get(bit)
}

pub fn set<const N: usize>(changed: &mut Chunk<N>, bit: usize) {
    changed.set(bit);
}

pub fn clear<const N: usize>(changed: &mut Chunk<N>, bit: usize) {
    if cfg!(panic = "unwind") {
        changed.clear(bit);
    }
}

#[diagnostic::on_unimplemented(
    message = "`{Self}` is not an array",
    label = "a bank must be an array `[T; N]`"
)]
pub trait Array {
    type Item;
    const LEN: usize;
}

impl<T, const N: usize> Array for [T; N] {
    type Item = T;
    const LEN: usize = N;
}

pub fn write<T: Eq + Clone, const N: usize>(
    bank: &mut [T],
    changed: &mut Chunk<N>,
    base: usize,
    write: Write<'_, T>,
) {
    write.apply(bank, |index| changed.set(base + index));
}

pub fn report<V, T, const N: usize>(
    bank: &[T],
    changed: &mut Chunk<N>,
    base: usize,
    visitor: &mut V,
) where
    V: for<'a> Visit<Element<'a, T>>,
{
    Element::report(bank, changed.bytes_mut(), base, visitor);
}

pub const fn input<T, C>(value: T, chain: C) -> (Input<T>, C) {
    (Input(value), chain)
}

pub const fn view<F, M, C>(f: F, chain: C) -> (View<F, M, F::Output>, C)
where
    F: ComputeImpl<C, M>,
{
    (
        View {
            f,
            marker: PhantomData,
        },
        chain,
    )
}

pub const fn bank<const BYTES: usize, T, const N: usize, C>(
    items: [T; N],
    chain: C,
) -> (Bank<T, N, BYTES>, C) {
    (Bank(items), chain)
}

pub fn unstore<T, C>((Input(value), chain): (Input<T>, C)) -> (T, C) {
    (value, chain)
}

pub fn unbank<T, const N: usize, const BYTES: usize, C>(
    (Bank(items), chain): (Bank<T, N, BYTES>, C),
) -> ([T; N], C) {
    (items, chain)
}

pub fn compute<F: ComputeImpl<C, M>, C, M>(f: &F, chain: &C) -> F::Output {
    f.compute(chain)
}

pub fn distinct<T, I, C: Select<T, I>>(_: &C) {}

/// A `strata!` struct that can be nested on the strata `S`: none of its
/// signals or fed types is also a signal or fed type of `S`. Implemented by
/// `strata!` with one index in `I` per signal and fed type.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be nested on these strata",
    label = "a type of this struct is also a signal or fed type of the formation",
    note = "only structs declared with `strata!` can be nested, and every signal and fed type must be distinct across the formation"
)]
pub trait Nestable<S, I> {}

/// The strata with `N` nested on `S`.
pub type Over<N, S> = (Nested<N>, S);

/// A check on the strata a struct is nested on.
pub trait Holds {}

/// Holds if `O` has a single signal `T`.
#[derive(Debug)]
pub struct Signals<O, T, I>(PhantomData<(O, T, I)>);

impl<O: Distinct<T, I>, T, I> Holds for Signals<O, T, I> {}

/// Holds if `O` accepts `T` in a single place.
#[derive(Debug)]
pub struct Feeds<O, T, I>(PhantomData<(O, T, I)>);

impl<O: Routed<T, I>, T, I> Holds for Feeds<O, T, I> {}

/// The check of an entry compiled out, holding for the index `()` alone.
#[derive(Debug)]
pub struct Skipped<I>(PhantomData<I>);

impl Holds for Skipped<()> {}

#[cfg(test)]
mod tests {
    use core::hint::black_box;

    use super::*;

    impl Signal<u8, ()> for () {
        const BIT: usize = 3;
        const CACHED: bool = true;
        type Ref<'a>
            = &'a u8
        where
            Self: 'a;

        fn value(&self) -> &u8 {
            &7
        }
    }

    #[test]
    fn signal_constants_are_read_through_the_helpers() {
        assert_eq!(bit::<(), u8, ()>(), 3);
        assert!(cached::<(), u8, ()>());
        assert_eq!(*value::<(), u8, ()>(&black_box(())), 7);
    }
}
