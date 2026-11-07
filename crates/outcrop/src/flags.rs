use crate::plumbing::Cursor;
use crate::plumbing::Flags;

#[derive(Debug, Default)]
pub struct Byte(u8);

impl Byte {
    pub(super) const EMPTY: Self = Self(0);
}

#[derive(Debug, Default)]
pub struct Chunk<Tail> {
    tail: Tail,
    byte: u8,
}

#[derive(Debug)]
pub struct Used<const BITS: u8>;

impl Flags for Byte {
    const BYTES: usize = 1;

    fn get(&self, index: usize) -> bool {
        self.0 & (1 << index) != 0
    }

    fn set_top(&mut self, index: usize) {
        self.0 |= 1 << index;
    }
}

impl<Tail: Flags> Flags for Chunk<Tail> {
    const BYTES: usize = Tail::BYTES + 1;

    fn get(&self, index: usize) -> bool {
        if index < 8 * Tail::BYTES {
            self.tail.get(index)
        } else {
            self.byte & (1 << (index % 8)) != 0
        }
    }

    fn set_top(&mut self, index: usize) {
        self.byte |= 1 << (index % 8);
    }
}

macro_rules! impl_partial {
    ($($used:literal => $next:literal),+) => {$(
        impl Cursor for Used<$used> {
            type Next = Used<$next>;
            type Grow<F: Flags> = F;

            fn grow<F: Flags>(flags: F) -> F {
                flags
            }

            fn below<F: Flags>(flags: &F) -> &F {
                flags
            }

            fn below_mut<F: Flags>(flags: &mut F) -> &mut F {
                flags
            }
        }
    )+};
}

impl_partial!(0 => 1, 1 => 2, 2 => 3, 3 => 4, 4 => 5, 5 => 6, 6 => 7, 7 => 8);

impl Cursor for Used<8> {
    type Next = Used<1>;
    type Grow<F: Flags> = Chunk<F>;

    fn grow<F: Flags>(flags: F) -> Chunk<F> {
        Chunk {
            tail: flags,
            byte: 0,
        }
    }

    fn below<F: Flags>(flags: &Chunk<F>) -> &F {
        &flags.tail
    }

    fn below_mut<F: Flags>(flags: &mut Chunk<F>) -> &mut F {
        &mut flags.tail
    }
}
