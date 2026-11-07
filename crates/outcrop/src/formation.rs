use core::marker::PhantomData;

use crate::flags::Byte;
use crate::plumbing::Cursor as _;
use crate::plumbing::Flags as _;
use crate::sink::Sink;
use crate::stratum::Compute;
use crate::stratum::Derived;
use crate::stratum::Feed;
use crate::stratum::Input;
use crate::stratum::Nil;
use crate::stratum::Report;
use crate::stratum::Router;
use crate::stratum::Strata;
use crate::stratum::Stratum;

type Deposited<E, S> = Formation<Stratum<E, S>>;

/// A statically shaped graph of signals.
///
/// Each [`input`](Self::input), [`with`](Self::with) or
/// [`route`](Self::route) deposits a stratum on top of the previous ones, and
/// a stratum can only depend on the strata beneath it, so cycles cannot be
/// written. Every signal has a value from the moment it is declared, and the
/// formation tracks changes in one bit per signal, sized from its strata.
///
/// Depending on a signal the formation does not contain is a compile-time
/// error:
///
/// ```compile_fail,E0277
/// # use outcrop::Formation;
/// #[derive(PartialEq)]
/// struct A(u8);
/// #[derive(PartialEq)]
/// struct B(u8);
/// let formation = Formation::new().with(|a: &A| B(a.0));
/// ```
///
/// And feeding a derived signal:
///
/// ```compile_fail,E0277
/// # use outcrop::Formation;
/// #[derive(PartialEq)]
/// struct A(u8);
/// #[derive(PartialEq)]
/// struct B(u8);
/// let mut formation = Formation::new().input(A(0)).with(|a: &A| B(a.0));
/// formation.feed(B(1));
/// ```
#[derive(Debug)]
pub struct Formation<S: Strata = Nil> {
    changed: S::Flags,
    strata: S,
}

impl Formation {
    /// Creates an empty formation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            changed: Byte::EMPTY,
            strata: Nil,
        }
    }
}

impl Default for Formation {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Strata> Formation<S> {
    /// Deposits an input signal with its initial value.
    ///
    /// The value is reported by the next [`poll`](Self::poll).
    #[must_use]
    pub fn input<T: PartialEq>(self, initial: T) -> Deposited<Input<T>, S> {
        let mut changed = S::Cursor::grow(self.changed);
        changed.set_top(S::LEN);
        Formation {
            changed,
            strata: Stratum {
                entry: Input(initial),
                tail: self.strata,
            },
        }
    }

    /// Deposits a signal derived by `f` from the signals beneath it.
    ///
    /// The dependencies are the argument types of `f`, so
    /// `|a: &A, b: &B| C(..)` derives a `C` from an `A` and a `B`. Its initial
    /// value is computed immediately and reported by the next
    /// [`poll`](Self::poll).
    #[must_use]
    pub fn with<F, M>(self, mut f: F) -> Deposited<Derived<F, M, F::Output>, S>
    where
        F: Compute<S, M>,
        F::Output: PartialEq,
    {
        let value = f.compute(&self.strata);
        let mut changed = S::Cursor::grow(self.changed);
        changed.set_top(S::LEN);
        Formation {
            changed,
            strata: Stratum {
                entry: Derived {
                    f,
                    value,
                    marker: PhantomData,
                },
                tail: self.strata,
            },
        }
    }

    /// Deposits a router that turns a fed `&In` into feeds of the inputs and
    /// routers beneath it.
    ///
    /// The routed value is not stored, so borrowed data such as byte slices
    /// can be fed, and several signals can be produced from a single feed.
    ///
    /// A router can only feed what lies beneath it:
    ///
    /// ```compile_fail,E0277
    /// # use outcrop::Formation;
    /// #[derive(PartialEq)]
    /// struct A(u8);
    /// #[derive(PartialEq)]
    /// struct B(u8);
    /// let formation = Formation::new()
    ///     .input(A(0))
    ///     .route(|a: &u8, out| out.feed(B(*a)));
    /// ```
    #[must_use]
    pub fn route<In: ?Sized, F>(self, f: F) -> Deposited<Router<In, F>, S>
    where
        F: FnMut(&In, &mut Sink<'_, S>),
    {
        Formation {
            changed: self.changed,
            strata: Stratum {
                entry: Router {
                    f,
                    marker: PhantomData,
                },
                tail: self.strata,
            },
        }
    }

    /// Feeds an input by value, or a router by reference.
    ///
    /// Feeding an input its current value changes nothing.
    pub fn feed<T, I>(&mut self, value: T)
    where
        S: Feed<T, I>,
    {
        self.strata.feed(&mut self.changed, value);
    }

    /// Recomputes the derived signals whose dependencies changed and reports
    /// every changed signal, inputs included, to `visitor`.
    ///
    /// A recomputed signal equal to its previous value is neither reported
    /// nor propagated to its dependents.
    pub fn poll<V>(&mut self, visitor: &mut V)
    where
        S: Report<V>,
    {
        self.strata.evaluate(&mut self.changed);
        self.strata
            .report(&core::mem::take(&mut self.changed), visitor);
    }
}
