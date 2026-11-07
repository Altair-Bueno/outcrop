use crate::stratum::Feed;
use crate::stratum::Strata;

/// Handle a router uses to feed the strata beneath it.
///
/// See [`Formation::route`](crate::Formation::route).
#[derive(Debug)]
pub struct Sink<'s, S: Strata> {
    pub(super) strata: &'s mut S,
    pub(super) changed: &'s mut S::Flags,
}

impl<S: Strata> Sink<'_, S> {
    /// Feeds an input or a router declared beneath the calling router.
    pub fn feed<T, I>(&mut self, value: T)
    where
        S: Feed<T, I>,
    {
        self.strata.feed(self.changed, value);
    }
}
