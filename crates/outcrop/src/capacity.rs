use bitmaps::Bitmap;
use bitmaps::Bits;
use bitmaps::BitsImpl;

use crate::plumbing::CapacityImpl;
use crate::plumbing::Flags;

/// The maximum number `N` of signals a [`Formation`](crate::Formation) holds.
#[derive(Debug)]
pub struct Capacity<const N: usize>;

/// Implemented by every capacity a formation supports, from 1 to 1024
/// signals.
///
/// Generic code over a formation of capacity `N` bounds it with
/// `Capacity<N>: Supported`. This trait is sealed.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a supported formation capacity",
    note = "a formation holds between 1 and 1024 signals"
)]
pub trait Supported: CapacityImpl {}

impl<C: CapacityImpl> Supported for C {}

impl<const N: usize> CapacityImpl for Capacity<N>
where
    BitsImpl<N>: Bits,
{
    type Flags = Bitmap<N>;
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
