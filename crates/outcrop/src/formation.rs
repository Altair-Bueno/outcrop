use core::marker::PhantomData;

use bitmaps::Bitmap;
use bitmaps::Bits;
use bitmaps::BitsImpl;

use crate::compute::Compute;
use crate::sink::Sink;
use crate::stratum::Derived;
use crate::stratum::Drain;
use crate::stratum::Evaluate;
use crate::stratum::Feed;
use crate::stratum::Input;
use crate::stratum::Len;
use crate::stratum::Nil;
use crate::stratum::Router;
use crate::stratum::Stratum;

type Deposited<const N: usize, E, S> = Formation<N, Stratum<E, S>>;

/// A statically shaped graph of signals holding at most `N` of them.
///
/// Each [`input`](Self::input), [`with`](Self::with) or
/// [`route`](Self::route) deposits a stratum on top of the previous ones, and
/// a stratum can only depend on the strata beneath it, so cycles cannot be
/// written. Every signal has a value from the moment it is declared.
///
/// Exceeding the capacity `N` is a compile-time error:
///
/// ```compile_fail,E0080
/// # use outcrop::Formation;
/// #[derive(PartialEq)]
/// struct A(u8);
/// #[derive(PartialEq)]
/// struct B(u8);
/// let formation = Formation::<1>::new().input(A(0)).input(B(0));
/// ```
///
/// So is depending on a signal the formation does not contain:
///
/// ```compile_fail,E0277
/// # use outcrop::Formation;
/// #[derive(PartialEq)]
/// struct A(u8);
/// #[derive(PartialEq)]
/// struct B(u8);
/// let formation = Formation::<8>::new().with(|a: &A| B(a.0));
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
/// let mut formation = Formation::<8>::new().input(A(0)).with(|a: &A| B(a.0));
/// formation.feed(B(1));
/// ```
#[derive(Debug)]
pub struct Formation<const N: usize, S = Nil>
where
    BitsImpl<N>: Bits,
{
    changed: Bitmap<N>,
    strata: S,
}

impl<const N: usize> Formation<N>
where
    BitsImpl<N>: Bits,
{
    /// Creates an empty formation.
    #[must_use]
    pub fn new() -> Self {
        Self {
            changed: Bitmap::new(),
            strata: Nil,
        }
    }
}

impl<const N: usize> Default for Formation<N>
where
    BitsImpl<N>: Bits,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize, S: Len> Formation<N, S>
where
    BitsImpl<N>: Bits,
{
    /// Deposits an input signal with its initial value.
    ///
    /// The value is reported by the next [`poll`](Self::poll).
    #[must_use]
    pub fn input<T: PartialEq>(mut self, initial: T) -> Deposited<N, Input<T>, S> {
        const {
            assert!(
                S::LEN < N,
                "the formation has more signals than its capacity N"
            );
        }
        self.changed.set(S::LEN, true);
        Formation {
            changed: self.changed,
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
    pub fn with<F, M>(mut self, mut f: F) -> Deposited<N, Derived<F, M, F::Output>, S>
    where
        F: Compute<S, M>,
        F::Output: PartialEq,
    {
        const {
            assert!(
                S::LEN < N,
                "the formation has more signals than its capacity N"
            );
        }
        let value = f.compute(&self.strata);
        self.changed.set(S::LEN, true);
        Formation {
            changed: self.changed,
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
    /// let formation = Formation::<8>::new()
    ///     .input(A(0))
    ///     .route(|a: &u8, out| out.feed(B(*a)));
    /// ```
    #[must_use]
    pub fn route<In: ?Sized, F>(self, f: F) -> Deposited<N, Router<In, F>, S>
    where
        F: FnMut(&In, &mut Sink<'_, S, Bitmap<N>>),
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
        S: Feed<T, I, Bitmap<N>>,
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
        S: Evaluate + Drain<V>,
    {
        self.strata.evaluate(&mut self.changed);
        self.strata
            .drain(&core::mem::take(&mut self.changed), visitor);
    }
}
