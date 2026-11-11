/// A signal value read with [`Formation::get`](crate::Formation::get),
/// tagged with whether it is up to date.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Reading<T> {
    /// No dependency of the signal changed since the last
    /// [`poll`](crate::Formation::poll), so the value is current.
    Fresh(T),
    /// A dependency of the signal changed since the last
    /// [`poll`](crate::Formation::poll), or the formation was never polled,
    /// so the next poll may recompute the value.
    Stale(T),
}

impl<T> Reading<T> {
    pub(crate) const fn new(value: T, stale: bool) -> Self {
        if stale {
            Self::Stale(value)
        } else {
            Self::Fresh(value)
        }
    }

    /// Returns the value, whether fresh or stale.
    pub fn into_inner(self) -> T {
        match self {
            Self::Fresh(value) | Self::Stale(value) => value,
        }
    }

    /// Returns `true` if the value is [`Fresh`](Self::Fresh).
    pub const fn is_fresh(&self) -> bool {
        matches!(self, Self::Fresh(_))
    }

    /// Returns `true` if the value is [`Stale`](Self::Stale).
    pub const fn is_stale(&self) -> bool {
        matches!(self, Self::Stale(_))
    }
}
