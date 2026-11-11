use crate::plumbing::Bits;
use crate::plumbing::Cursor;

#[derive(Debug)]
pub struct Used<const BITS: u8>;

impl Bits for u8 {
    const BITS: usize = 8;
    const ALL: Self = Self::MAX;

    fn get(&self, index: usize) -> bool {
        self & (1 << index) != 0
    }

    fn set_top(&mut self, index: usize) {
        *self |= 1 << index;
    }

    fn clear_top(&mut self, index: usize) {
        *self &= !(1 << index);
    }

    fn byte(&self, _: usize) -> u8 {
        *self
    }
}

impl<Tail: Bits, Top: Bits> Bits for (Tail, Top) {
    const BITS: usize = Tail::BITS + Top::BITS;
    const ALL: Self = (Tail::ALL, Top::ALL);

    fn get(&self, index: usize) -> bool {
        if index < Tail::BITS {
            self.0.get(index)
        } else {
            self.1.get(index - Tail::BITS)
        }
    }

    fn set_top(&mut self, index: usize) {
        self.1.set_top(index - Tail::BITS);
    }

    fn clear_top(&mut self, index: usize) {
        self.1.clear_top(index - Tail::BITS);
    }

    fn byte(&self, index: usize) -> u8 {
        if index < Tail::BITS / 8 {
            self.0.byte(index)
        } else {
            self.1.byte(index - Tail::BITS / 8)
        }
    }
}

/// Change bits packed in `BYTES` bytes, one per bank element or `strata!`
/// signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chunk<const BYTES: usize>([u8; BYTES]);

impl<const BYTES: usize> Chunk<BYTES> {
    pub(crate) const ALL: Self = Self([u8::MAX; BYTES]);

    pub(crate) fn get(&self, index: usize) -> bool {
        self.0[index / 8].get(index % 8)
    }

    pub(crate) fn set(&mut self, index: usize) {
        self.0[index / 8].set_top(index % 8);
    }

    pub(crate) fn clear(&mut self, index: usize) {
        self.0[index / 8].clear_top(index % 8);
    }

    pub(crate) const fn bytes_mut(&mut self) -> &mut [u8; BYTES] {
        &mut self.0
    }
}

#[cfg(feature = "macros")]
impl<const BYTES: usize> Chunk<BYTES> {
    /// Only the bit `index` set.
    pub(crate) const fn bit(index: usize) -> Self {
        let mut bit = [0; BYTES];
        bit[index / 8] = 1 << (index % 8);
        Self(bit)
    }

    /// Every bit set in any of `chunks`.
    pub(crate) const fn union<const M: usize>(chunks: [Self; M]) -> Self {
        let mut union = [0; BYTES];
        let mut chunk = 0;
        while chunk < M {
            let mut byte = 0;
            while byte < BYTES {
                union[byte] |= chunks[chunk].0[byte];
                byte += 1;
            }
            chunk += 1;
        }
        Self(union)
    }

    pub(crate) const fn bytes(&self) -> &[u8; BYTES] {
        &self.0
    }

    /// Whether `self` and `other` share any set bit.
    pub(crate) fn intersects(&self, other: &Self) -> bool {
        self.0.iter().zip(&other.0).any(|(a, b)| a & b != 0)
    }
}

impl<const BYTES: usize> Default for Chunk<BYTES> {
    fn default() -> Self {
        Self([0; BYTES])
    }
}

impl<const BYTES: usize> Bits for Chunk<BYTES> {
    const BITS: usize = 8 * BYTES;
    const ALL: Self = Self([u8::MAX; BYTES]);

    fn get(&self, index: usize) -> bool {
        Self::get(self, index)
    }

    fn set_top(&mut self, index: usize) {
        self.set(index);
    }

    fn clear_top(&mut self, index: usize) {
        self.clear(index);
    }

    fn byte(&self, index: usize) -> u8 {
        self.0[index]
    }
}

macro_rules! impl_partial {
    ($($used:literal => $next:literal),+) => {$(
        impl Cursor for Used<$used> {
            const SPARE: usize = 8 - $used;
            type Next = Used<$next>;
            type Grow<F: Bits> = F;

            fn grow<F: Bits>(flags: F) -> F {
                flags
            }

            fn below_mut<F: Bits>(flags: &mut F) -> &mut F {
                flags
            }
        }
    )+};
}

impl_partial!(0 => 1, 1 => 2, 2 => 3, 3 => 4, 4 => 5, 5 => 6, 6 => 7, 7 => 8);

impl Cursor for Used<8> {
    const SPARE: usize = 0;
    type Next = Used<1>;
    type Grow<F: Bits> = (F, u8);

    fn grow<F: Bits>(flags: F) -> (F, u8) {
        (flags, 0)
    }

    fn below_mut<F: Bits>(flags: &mut (F, u8)) -> &mut F {
        &mut flags.0
    }
}
