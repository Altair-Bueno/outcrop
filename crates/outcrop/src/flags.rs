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
}

impl<Tail: Bits> Bits for (Tail, u8) {
    const BITS: usize = Tail::BITS + 8;
    const ALL: Self = (Tail::ALL, u8::MAX);

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
}

/// The change bits of a bank, one per element, packed in `BYTES` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chunk<const BYTES: usize>([u8; BYTES]);

impl<const BYTES: usize> Chunk<BYTES> {
    pub(crate) const ALL: Self = Self([u8::MAX; BYTES]);

    pub(crate) const fn bytes(&self) -> &[u8; BYTES] {
        &self.0
    }

    pub(crate) fn get(&self, index: usize) -> bool {
        self.0[index / 8].get(index % 8)
    }

    pub(crate) fn set(&mut self, index: usize) {
        self.0[index / 8].set_top(index % 8);
    }
}

impl<const BYTES: usize> Default for Chunk<BYTES> {
    fn default() -> Self {
        Self([0; BYTES])
    }
}

impl<Tail: Bits, const BYTES: usize> Bits for (Tail, Chunk<BYTES>) {
    const BITS: usize = Tail::BITS + 8 * BYTES;
    const ALL: Self = (Tail::ALL, Chunk::ALL);

    fn get(&self, index: usize) -> bool {
        if index < Tail::BITS {
            self.0.get(index)
        } else {
            self.1.get(index - Tail::BITS)
        }
    }

    fn set_top(&mut self, index: usize) {
        self.1.set(index - Tail::BITS);
    }
}

impl<Tail: Bits, const BYTES: usize> Bits for (Tail, [u8; BYTES])
where
    [u8; BYTES]: Default,
{
    const BITS: usize = Tail::BITS + 8 * BYTES;
    const ALL: Self = (Tail::ALL, [u8::MAX; BYTES]);

    fn get(&self, index: usize) -> bool {
        if index < Tail::BITS {
            self.0.get(index)
        } else {
            let index = index - Tail::BITS;
            self.1[index / 8].get(index % 8)
        }
    }

    fn set_top(&mut self, index: usize) {
        let index = index - Tail::BITS;
        self.1[index / 8].set_top(index % 8);
    }
}

macro_rules! impl_partial {
    ($($used:literal => $next:literal),+) => {$(
        impl Cursor for Used<$used> {
            type Next = Used<$next>;
            type Grow<F: Bits> = F;

            fn grow<F: Bits>(flags: F) -> F {
                flags
            }

            fn below<F: Bits>(flags: &F) -> &F {
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
    type Next = Used<1>;
    type Grow<F: Bits> = (F, u8);

    fn grow<F: Bits>(flags: F) -> (F, u8) {
        (flags, 0)
    }

    fn below<F: Bits>(flags: &(F, u8)) -> &F {
        &flags.0
    }

    fn below_mut<F: Bits>(flags: &mut (F, u8)) -> &mut F {
        &mut flags.0
    }
}
