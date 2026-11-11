use crate::visit::Visit;

/// A signal stored in one element of a bank.
///
/// A bank, deposited with [`Formation::bank`](crate::Formation::bank) or
/// `#[strata(bank)]`, holds an array `[Item; N]` and tracks a change bit per
/// element. Each element is read as the signal type whose `INDEX` points at
/// it, so a memo depending on `&Self` is recomputed only when that element
/// changed. `INDEX` must be below `N`, which is checked when the code using
/// the slot is compiled to machine code: by `cargo build`, not `cargo check`.
///
/// ```
/// use outcrop::Slot;
///
/// struct Register<const ADDRESS: usize>(u16);
///
/// impl<const ADDRESS: usize> Slot for Register<ADDRESS> {
///     type Item = u16;
///     const INDEX: usize = ADDRESS;
///
///     fn read(item: &u16) -> Self {
///         Self(*item)
///     }
/// }
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a slot of a bank",
    note = "implement `Slot` to read `{Self}` from an element of a bank"
)]
pub trait Slot: Sized {
    /// The type of the bank's elements.
    type Item;

    /// The element of the bank this signal is read from.
    const INDEX: usize;

    /// Reads the signal from its element.
    fn read(item: &Self::Item) -> Self;
}

/// Consecutive elements written to a bank, starting at `start`.
///
/// Feeding a `Write` compares each element with the one in the bank and marks
/// only those that differ as changed, until the next poll, as feeding an input
/// does. Elements past the end of the bank are ignored.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Write<'a, T> {
    start: usize,
    items: &'a [T],
}

impl<'a, T> Write<'a, T> {
    /// Writes `items` to the elements starting at `start`.
    #[must_use]
    pub const fn new(start: usize, items: &'a [T]) -> Self {
        Self { start, items }
    }

    /// Writes each item over the element at its index in `bank`, calling
    /// `changed` with the index of every element that differed.
    pub(crate) fn apply(self, bank: &mut [T], mut changed: impl FnMut(usize))
    where
        T: Eq + Clone,
    {
        let elements = bank.iter_mut().enumerate().skip(self.start);
        for ((index, element), item) in elements.zip(self.items) {
            if element != item {
                element.clone_from(item);
                changed(index);
            }
        }
    }
}

/// An element of a bank that changed, as reported by
/// [`Formation::poll`](crate::Formation::poll).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Element<'a, T> {
    /// The position of the element in the bank.
    pub index: usize,
    /// Its current value.
    pub value: &'a T,
}

impl<'a, T> Element<'a, T> {
    /// Visits every element of `bank` whose change bit is set, the bits
    /// starting at bit `start` of `bits`, clearing each bit before visiting
    /// its element under `panic = "unwind"`. Bytes with no bit set are skipped whole.
    pub(crate) fn report<V>(bank: &'a [T], bits: &mut [u8], start: usize, visitor: &mut V)
    where
        V: Visit<Self>,
    {
        let end = start + bank.len();
        let mut byte = start / 8;
        while 8 * byte < end {
            let mut set = bits[byte];
            if 8 * byte < start {
                set &= u8::MAX << (start % 8);
            }
            if 8 * byte + 8 > end {
                set &= u8::MAX >> (8 * byte + 8 - end);
            }
            while set != 0 {
                let low = usize::try_from(set.trailing_zeros()).unwrap_or_default();
                set &= set - 1;
                if cfg!(panic = "unwind") {
                    bits[byte] &= !(1 << low);
                }
                let index = 8 * byte + low - start;
                visitor.visit(&Self {
                    index,
                    value: &bank[index],
                });
            }
            byte += 1;
        }
    }
}
