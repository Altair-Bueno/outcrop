use crate::strata::Feed;
use crate::strata::Strata;

/// Handle a router uses to feed the strata beneath it.
///
/// See [`Formation::route`](crate::Formation::route).
#[derive(Debug)]
pub struct Sink<'s, S: Strata> {
    strata: &'s mut S,
    changed: &'s mut S::Flags,
}

impl<'s, S: Strata> Sink<'s, S> {
    /// Creates a sink feeding `strata`, marking changes in `changed`.
    ///
    /// Only needed to implement a router by hand; routers declared with
    /// [`Formation::route`](crate::Formation::route) or `strata!` receive
    /// theirs.
    #[must_use]
    pub const fn new(strata: &'s mut S, changed: &'s mut S::Flags) -> Self {
        Self { strata, changed }
    }

    /// Feeds an input by value, or a router by reference, declared beneath
    /// the calling router.
    pub fn feed<T, I>(&mut self, value: T)
    where
        S: Feed<T, I>,
    {
        self.strata.feed(self.changed, value);
    }
}
