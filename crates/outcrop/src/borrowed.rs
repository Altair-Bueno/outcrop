use crate::bank::Slot;
use crate::flags::Chunk;
use crate::flags::Used;
use crate::plumbing::Bits;
use crate::plumbing::Layer;
use crate::plumbing::LayeredImpl;
use crate::plumbing::Select;
use crate::plumbing::Settle;
use crate::plumbing::Value;
use crate::strata::Strata;
use crate::stratum::Slotted;
use crate::stratum::read;
use crate::stratum::slot;

/// A signal of a `strata!` struct under construction, borrowed from the local
/// holding it, so that building the chain moves a pointer instead of the value.
#[derive(Debug, Clone, Copy)]
pub struct Borrowed<'a, T>(pub(super) &'a T);

/// A bank of a `strata!` struct under construction, borrowed from the local
/// holding it.
#[derive(Debug, Clone, Copy)]
pub struct BorrowedBank<'a, T, const N: usize, const BYTES: usize>(pub(super) &'a [T; N]);

type Banked<Tail, const BYTES: usize> = (<Tail as Strata>::Flags, Chunk<BYTES>);

impl<T> Value for Borrowed<'_, T> {
    type Signal = T;
}

impl<T, Tail> Settle<Tail> for Borrowed<'_, T> {
    const CACHED: bool = false;
    type Ref<'a>
        = &'a T
    where
        Self: 'a,
        Tail: 'a;

    fn value<'a>(&'a self, _: &'a Tail) -> &'a T {
        self.0
    }

    fn settle<Fl: Bits>(&mut self, _: &Tail, _: &Fl) -> bool {
        false
    }
}

impl<T, const N: usize, const BYTES: usize, Tail> Layer<Tail> for BorrowedBank<'_, T, N, BYTES>
where
    Tail: LayeredImpl,
{
    type Flags = Banked<Tail, BYTES>;
    type Cursor = Used<8>;

    fn deposit(flags: Tail::Flags) -> Self::Flags {
        (flags, Chunk::ALL)
    }

    fn below_mut(flags: &mut Self::Flags) -> &mut Tail::Flags {
        &mut flags.0
    }

    fn evaluate(&mut self, _: &Tail, _: &mut Self::Flags) {}
}

impl<E, const N: usize, const BYTES: usize, Tail> Select<E, Slotted>
    for (BorrowedBank<'_, E::Item, N, BYTES>, Tail)
where
    E: Slot,
    Tail: LayeredImpl,
{
    const INDEX: usize = slot::<E, N>(<Tail::Flags as Bits>::BITS);
    const CACHED: bool = false;
    type Ref<'a>
        = E
    where
        Self: 'a;

    fn select(&self) -> E {
        read(self.0.0)
    }
}
