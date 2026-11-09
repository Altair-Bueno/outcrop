use crate::__private::Signal;
use crate::flags::Chunk;
use crate::flags::Used;
use crate::plumbing::Bits;
use crate::plumbing::Layer;
use crate::plumbing::LayerReport;
use crate::plumbing::LayeredImpl;
use crate::plumbing::Select;
use crate::strata::Feed;
use crate::strata::Report;
use crate::strata::Strata;

/// A struct declared with `strata!`, deposited as one layer of a formation.
#[derive(Debug, Clone, Copy)]
pub struct Nested<S>(pub(super) S);

/// Index of a signal of a nested struct in [`Select`] and [`Feed`], wrapping
/// the index the struct finds it by.
#[derive(Debug)]
pub struct Within<I>(I);

type Stacked<Tail, S> = (<Tail as Strata>::Flags, <S as Strata>::Flags);

impl<S, Tail> Layer<Tail> for Nested<S>
where
    S: Strata,
    Tail: LayeredImpl,
    Stacked<Tail, S>: Bits,
{
    type Flags = Stacked<Tail, S>;
    type Cursor = Used<8>;

    fn deposit(flags: Tail::Flags) -> Self::Flags {
        (flags, S::CHANGED)
    }

    fn below(flags: &Self::Flags) -> &Tail::Flags {
        &flags.0
    }

    fn below_mut(flags: &mut Self::Flags) -> &mut Tail::Flags {
        &mut flags.0
    }

    fn evaluate(&mut self, _: &Tail, changed: &mut Self::Flags) {
        self.0.evaluate(&mut changed.1);
    }
}

impl<V, S, Tail> LayerReport<V, Tail> for Nested<S>
where
    S: Report<V>,
    Tail: LayeredImpl,
    Stacked<Tail, S>: Bits,
{
    fn report(&self, _: &Tail, changed: &mut Self::Flags, visitor: &mut V) {
        self.0.report(&mut changed.1, visitor);
    }
}

impl<T, I, S, Tail, const N: usize> Select<T, Within<I>> for (Nested<S>, Tail)
where
    S: Signal<T, I, Flags = Chunk<N>>,
    Tail: LayeredImpl,
{
    const INDEX: usize = <Tail::Flags as Bits>::BITS + S::BIT;
    type Ref<'a>
        = S::Ref<'a>
    where
        Self: 'a;

    fn select(&self) -> S::Ref<'_> {
        self.0.0.value()
    }

    fn stale<Fl: Bits>(changed: &Fl) -> bool {
        intersects::<Tail, Fl>(changed, S::STALE.bytes())
    }

    fn moved<Fl: Bits>(changed: &Fl) -> bool {
        intersects::<Tail, Fl>(changed, S::MOVED.bytes())
    }
}

impl<T, I, S, Tail> Feed<T, Within<I>> for (Nested<S>, Tail)
where
    S: Feed<T, I>,
    Tail: LayeredImpl,
    Stacked<Tail, S>: Bits,
{
    fn feed(&mut self, changed: &mut Self::Flags, value: T) {
        self.0.0.feed(&mut changed.1, value);
    }
}

/// Whether `changed` has any bit of `mask` set, `mask` being the flags of a
/// struct nested on `Tail`.
fn intersects<Tail: LayeredImpl, Fl: Bits>(changed: &Fl, mask: &[u8]) -> bool {
    let base = <Tail::Flags as Bits>::BITS;
    (0..8 * mask.len()).any(|bit| mask[bit / 8].get(bit % 8) && changed.get(base + bit))
}
