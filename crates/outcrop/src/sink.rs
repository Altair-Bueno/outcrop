use crate::capacity::Supported;
use crate::plumbing::CapacityImpl;
use crate::stratum::Feed;

/// Handle a router uses to feed the strata beneath it.
///
/// See [`Formation::route`](crate::Formation::route).
#[derive(Debug)]
pub struct Sink<'s, S, C: Supported> {
    pub(super) strata: &'s mut S,
    pub(super) changed: &'s mut <C as CapacityImpl>::Flags,
}

impl<S, C: Supported> Sink<'_, S, C> {
    /// Feeds an input or a router declared beneath the calling router.
    pub fn feed<T, I>(&mut self, value: T)
    where
        S: Feed<T, I, C>,
    {
        self.strata.feed(self.changed, value);
    }
}
