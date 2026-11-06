use bitmaps::Bitmap;
use bitmaps::Bits;
use bitmaps::BitsImpl;

/// One "changed since the last poll" bit per signal of a
/// [`Formation`](crate::Formation).
///
/// A signal's index is its position in the formation, counted from the first
/// stratum.
pub trait Flags {
    /// Returns whether the signal at `index` changed.
    fn get(&self, index: usize) -> bool;

    /// Marks the signal at `index` as changed.
    fn set(&mut self, index: usize);
}

impl<const N: usize> Flags for Bitmap<N>
where
    BitsImpl<N>: Bits,
{
    fn get(&self, index: usize) -> bool {
        Self::get(*self, index)
    }

    fn set(&mut self, index: usize) {
        Self::set(self, index, true);
    }
}
