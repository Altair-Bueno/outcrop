//! Support for the code `strata!` generates. Not public API.

use core::borrow::Borrow;
use core::marker::PhantomData;

use crate::bank::Element;
use crate::bank::Slot;
use crate::bank::Write;
use crate::plumbing::ComputeImpl;
use crate::plumbing::Select;
use crate::reading::Reading;
use crate::strata::Feed;
use crate::strata::Strata;
use crate::stratum::Bank;
use crate::stratum::Input;
use crate::stratum::View;
use crate::visit::Visit;

pub trait Signal<T, I>: Strata {
    const BIT: usize;
    const STALE: Self::Flags;
    const MOVED: Self::Flags;
    type Ref<'a>: Borrow<T>
    where
        Self: 'a;

    fn value(&self) -> Self::Ref<'_>;
}

pub const fn bit<S: Signal<T, I>, T, I>() -> usize {
    S::BIT
}

pub const fn stale<S: Signal<T, I>, T, I>() -> S::Flags {
    S::STALE
}

pub const fn moved<S: Signal<T, I>, T, I>() -> S::Flags {
    S::MOVED
}

pub fn value<S: Signal<T, I>, T, I>(strata: &S) -> S::Ref<'_> {
    strata.value()
}

pub fn get<'s, S, T, I, const N: usize>(strata: &'s S, changed: &[u8; N]) -> Reading<S::Ref<'s>>
where
    S: Signal<T, I, Flags = [u8; N]>,
{
    let value = strata.value();
    if intersects(changed, &S::STALE) {
        Reading::Stale(value)
    } else {
        Reading::Fresh(value)
    }
}

pub fn routed<S: Feed<T, I>, T, I>() {}

pub const fn mask<const N: usize>(bit: usize) -> [u8; N] {
    let mut mask = [0; N];
    mask[bit / 8] = 1 << (bit % 8);
    mask
}

pub const fn union<const N: usize, const M: usize>(masks: [[u8; N]; M]) -> [u8; N] {
    let mut union = [0; N];
    let mut mask = 0;
    while mask < M {
        let mut byte = 0;
        while byte < N {
            union[byte] |= masks[mask][byte];
            byte += 1;
        }
        mask += 1;
    }
    union
}

pub fn intersects<const N: usize>(changed: &[u8; N], mask: &[u8; N]) -> bool {
    changed
        .iter()
        .zip(mask)
        .any(|(changed, mask)| changed & mask != 0)
}

pub const fn is_set<const N: usize>(changed: &[u8; N], bit: usize) -> bool {
    changed[bit / 8] & 1 << (bit % 8) != 0
}

pub const fn set<const N: usize>(changed: &mut [u8; N], bit: usize) {
    changed[bit / 8] |= 1 << (bit % 8);
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

pub const fn slot<E: Slot>(base: usize) -> usize {
    base + E::INDEX
}

/// Reads the slot `E` from `bank`, rejecting at compile time a slot past its
/// end.
pub fn read<E: Slot, const N: usize>(bank: &[E::Item; N]) -> E {
    const { assert!(E::INDEX < N, "the slot is past the end of the bank") };
    E::read(&bank[E::INDEX])
}

pub fn write<T: PartialEq + Clone, const N: usize>(
    bank: &mut [T],
    changed: &mut [u8; N],
    base: usize,
    write: Write<'_, T>,
) {
    write.apply(bank, |index| set(changed, base + index));
}

pub fn report<V, T, const N: usize>(bank: &[T], changed: &[u8; N], base: usize, visitor: &mut V)
where
    V: for<'a> Visit<Element<'a, T>>,
{
    Element::report(bank, changed, base, visitor);
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

pub fn select<T, I, C: Select<T, I>>(chain: &C) -> C::Ref<'_> {
    chain.select()
}

pub fn distinct<T, I, C: Select<T, I>>(_: &C) {}

#[cfg(test)]
mod tests {
    use core::hint::black_box;

    use super::*;

    impl Signal<u8, ()> for () {
        const BIT: usize = 3;
        const STALE: u8 = 0b0000_1000;
        const MOVED: u8 = 0b0000_1010;
        type Ref<'a>
            = &'a u8
        where
            Self: 'a;

        fn value(&self) -> &u8 {
            &7
        }
    }

    #[test]
    fn masks_set_one_bit_in_the_byte_that_holds_it() {
        assert_eq!(mask::<2>(black_box(0)), [0b0000_0001, 0]);
        assert_eq!(mask::<2>(black_box(7)), [0b1000_0000, 0]);
        assert_eq!(mask::<2>(black_box(8)), [0, 0b0000_0001]);
        assert_eq!(mask::<2>(black_box(9)), [0, 0b0000_0010]);
    }

    #[test]
    fn unions_hold_every_bit_of_every_mask() {
        let masks = black_box([mask::<2>(3), mask(9), mask(3)]);

        assert_eq!(union(masks), [0b0000_1000, 0b0000_0010]);
        assert_eq!(union::<2, 0>(black_box([])), [0, 0]);
    }

    #[test]
    fn masks_intersect_when_they_share_a_bit_in_any_byte() {
        let changed = black_box([0b0000_0100, 0b0000_0010]);

        assert!(intersects(&changed, &black_box(mask(9))));
        assert!(intersects(&changed, &black_box(mask(2))));
        assert!(!intersects(&changed, &black_box(mask(3))));
        assert!(!intersects(&changed, &black_box([0, 0])));
    }

    #[test]
    fn signal_constants_are_read_through_the_helpers() {
        assert_eq!(bit::<(), u8, ()>(), 3);
        assert_eq!(stale::<(), u8, ()>(), 0b0000_1000);
        assert_eq!(moved::<(), u8, ()>(), 0b0000_1010);
        assert_eq!(*value::<(), u8, ()>(&black_box(())), 7);
    }
}
