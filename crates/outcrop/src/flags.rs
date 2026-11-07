use crate::plumbing::Bits;
use crate::plumbing::Cursor;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Byte(u8);

impl Byte {
    pub(super) const EMPTY: Self = Self(0);
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Chunk<Tail> {
    tail: Tail,
    byte: Byte,
}

#[derive(Debug)]
pub struct Used<const BITS: u8>;

impl Bits for Byte {
    const BYTES: usize = 1;
    const ALL: Self = Self(u8::MAX);

    fn get(&self, index: usize) -> bool {
        self.0 & (1 << (index % 8)) != 0
    }

    fn set_top(&mut self, index: usize) {
        self.0 |= 1 << (index % 8);
    }
}

impl<Tail: Bits> Bits for Chunk<Tail> {
    const BYTES: usize = Tail::BYTES + 1;
    const ALL: Self = Self {
        tail: Tail::ALL,
        byte: Byte::ALL,
    };

    fn get(&self, index: usize) -> bool {
        if index < 8 * Tail::BYTES {
            self.tail.get(index)
        } else {
            self.byte.get(index)
        }
    }

    fn set_top(&mut self, index: usize) {
        self.byte.set_top(index);
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
    type Grow<F: Bits> = Chunk<F>;

    fn grow<F: Bits>(flags: F) -> Chunk<F> {
        Chunk {
            tail: flags,
            byte: Byte::EMPTY,
        }
    }

    fn below<F: Bits>(flags: &Chunk<F>) -> &F {
        &flags.tail
    }

    fn below_mut<F: Bits>(flags: &mut Chunk<F>) -> &mut F {
        &mut flags.tail
    }
}
