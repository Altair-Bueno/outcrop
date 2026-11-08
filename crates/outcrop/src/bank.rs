/// A signal stored in one element of a bank.
///
/// A bank, deposited with [`Formation::bank`](crate::Formation::bank) or
/// `#[strata(bank)]`, holds an array `[Item; N]` and tracks a change bit per
/// element. Each element is read as the signal type whose `INDEX` points at
/// it, so a memo depending on `&Self` is recomputed only when that element
/// changed. `INDEX` must be below `N`, which is checked at compile time.
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
/// only those that differ as changed. Elements past the end of the bank are
/// ignored.
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
        T: PartialEq + Clone,
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
