use core::marker::PhantomData;

use crate::bank::Write;
use crate::flags::Chunk;
use crate::plumbing::Bits as _;
use crate::plumbing::Cursor as _;
use crate::plumbing::Distinct;
use crate::plumbing::Routed;
use crate::reading::Reading;
use crate::sink::Sink;
use crate::strata::Feed;
use crate::strata::Get;
use crate::strata::Report;
use crate::strata::Strata;
use crate::stratum::Bank;
use crate::stratum::Compute;
use crate::stratum::Input;
use crate::stratum::Layered;
use crate::stratum::Memo;
use crate::stratum::Router;
use crate::stratum::View;

type Deposited<E, S> = Formation<(E, S)>;

/// A statically shaped graph of signals.
///
/// Each [`input`](Self::input), [`memo`](Self::memo), [`view`](Self::view)
/// or [`route`](Self::route) deposits a stratum on top of the previous ones,
/// and
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
/// let formation = Formation::new().memo(|a: &A| B(a.0));
/// ```
///
/// And feeding a memo or a view:
///
/// ```compile_fail,E0277
/// # use outcrop::Formation;
/// #[derive(PartialEq)]
/// struct A(u8);
/// #[derive(PartialEq)]
/// struct B(u8);
/// let mut formation = Formation::new().input(A(0)).view(|a: &A| B(a.0));
/// formation.feed(B(1));
/// ```
///
/// Every signal has its own type, so declaring a type twice is rejected:
///
/// ```compile_fail
/// # use outcrop::Formation;
/// #[derive(PartialEq)]
/// struct A(u8);
/// let formation = Formation::new().input(A(0)).input(A(1));
/// ```
///
/// And so is routing a type twice:
///
/// ```compile_fail
/// # use outcrop::Formation;
/// let formation = Formation::new().route(|_: &u8, _| {}).route(|_: &u8, _| {});
/// ```
///
/// Finding a signal by its type takes one level of trait resolution per
/// signal above it, so a formation of around 120 signals or more exceeds the
/// compiler's default recursion limit of 128. The compiler reports it as
/// E0275, "overflow evaluating the requirement", and newer compilers already
/// warn that exceeding it will become a hard error. Raise the limit in the
/// crate that builds the formation, with `#![recursion_limit = "256"]`.
/// `strata!` is subject to the same limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Formation<S: Strata = ()> {
    changed: S::Flags,
    strata: S,
}

impl Formation {
    /// Creates an empty formation.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            changed: 0,
            strata: (),
        }
    }
}

impl Default for Formation {
    fn default() -> Self {
        Self::new()
    }
}

impl<S: Strata> From<S> for Formation<S> {
    /// Wraps `strata` with every signal marked as changed, so that the first
    /// [`poll`](Self::poll) reports every initial value.
    fn from(strata: S) -> Self {
        Self {
            changed: S::CHANGED,
            strata,
        }
    }
}

impl<S: Layered> Formation<S> {
    /// Deposits an input signal with its initial value.
    ///
    /// The value is reported by the next [`poll`](Self::poll).
    #[must_use]
    pub fn input<T: PartialEq, I>(self, initial: T) -> Deposited<Input<T>, S>
    where
        (Input<T>, S): Distinct<T, I>,
    {
        let mut changed = S::Cursor::grow(self.changed);
        changed.set_top(S::LEN);
        Formation {
            changed,
            strata: (Input(initial), self.strata),
        }
    }

    /// Deposits a bank: an array of inputs with a change bit per element,
    /// each read as the [`Slot`](crate::Slot) whose index points at it.
    ///
    /// A bank is fed a [`Write`], which marks only the elements whose value
    /// changed, so a signal depending on a slot is recomputed and reported
    /// only when its element changed. [`poll`](Self::poll) reports every
    /// changed element as an [`Element`](crate::Element), and the next one
    /// reports them all.
    ///
    /// Each element costs a byte of flags, and the whole bank one level of
    /// the recursion limit, however many elements it holds.
    ///
    /// ```
    /// use outcrop::{Formation, Slot, Write};
    ///
    /// #[derive(Debug, PartialEq)]
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
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Celsius(u16);
    ///
    /// let mut formation = Formation::new()
    ///     .bank([0_u16; 4])
    ///     .memo(|r: &Register<2>| Celsius(r.0 / 10));
    ///
    /// formation.feed(Write::new(1, &[7, 215]));
    /// formation.poll(&mut ());
    /// assert_eq!(*formation.get::<Celsius, _>().into_inner(), Celsius(21));
    /// ```
    ///
    /// Two banks of the same item type are rejected, since a `Write` could
    /// not tell them apart:
    ///
    /// ```compile_fail
    /// # use outcrop::Formation;
    /// let formation = Formation::new().bank([0_u16; 2]).bank([0_u16; 3]);
    /// ```
    #[must_use]
    pub fn bank<T: PartialEq + Clone, const N: usize, I>(
        self,
        initial: [T; N],
    ) -> Deposited<Bank<T, N>, S>
    where
        for<'w> (Bank<T, N>, S): Routed<Write<'w, T>, I>,
    {
        Formation {
            changed: (self.changed, Chunk([true; N])),
            strata: (Bank(initial), self.strata),
        }
    }

    /// Deposits a memo: a signal computed by `f` from the signals beneath it
    /// and cached, for values that are expensive to compute.
    ///
    /// The dependencies are the argument types of `f`, so
    /// `|a: &A, b: &B| C(..)` computes a `C` from an `A` and a `B`, which may
    /// be inputs, memos or views. The initial value is computed immediately
    /// and reported by the next [`poll`](Self::poll), which recomputes it
    /// whenever a dependency changed.
    ///
    /// `f` must depend only on its arguments. A memo is recomputed when a
    /// dependency changes and at no other time, so nothing else `f` reads is
    /// tracked: a captured atomic or `Cell` that changes behind the
    /// formation's back leaves the memo out of date, which is a logic error
    /// the formation cannot detect. Values that vary belong in inputs.
    #[must_use]
    pub fn memo<F, M, I>(self, f: F) -> Deposited<Memo<F, M, F::Output>, S>
    where
        F: Compute<S, M>,
        F::Output: PartialEq,
        (Memo<F, M, F::Output>, S): Distinct<F::Output, I>,
    {
        let value = f.compute(&self.strata);
        let mut changed = S::Cursor::grow(self.changed);
        changed.set_top(S::LEN);
        Formation {
            changed,
            strata: (
                Memo {
                    f,
                    value,
                    marker: PhantomData,
                },
                self.strata,
            ),
        }
    }

    /// Deposits a view: a signal computed by `f` from the signals beneath it
    /// every time it is read, for values cheap enough not to be stored.
    ///
    /// The dependencies are inferred as for [`memo`](Self::memo). A view
    /// takes no memory of its own and is computed again on every
    /// [`get`](Self::get), report and computation of the signals above it.
    /// Its value is never compared, so it needs no `PartialEq`:
    /// [`poll`](Self::poll) reports it whenever a dependency changed, even if
    /// its own value did not, and memos above it are then recomputed.
    ///
    /// As for a memo, `f` must depend only on its arguments: [`get`](Self::get)
    /// computes `f` afresh, but [`poll`](Self::poll) only reports a view when
    /// a dependency changed, so a change in anything else `f` reads goes
    /// unreported.
    #[must_use]
    pub fn view<F, M, I>(self, f: F) -> Deposited<View<F, M, F::Output>, S>
    where
        F: Compute<S, M>,
        (View<F, M, F::Output>, S): Distinct<F::Output, I>,
    {
        let mut changed = S::Cursor::grow(self.changed);
        changed.set_top(S::LEN);
        Formation {
            changed,
            strata: (
                View {
                    f,
                    marker: PhantomData,
                },
                self.strata,
            ),
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
    pub fn route<In: ?Sized, F, I>(self, f: F) -> Deposited<Router<In, F>, S>
    where
        F: FnMut(&In, &mut Sink<'_, S>),
        for<'a> (Router<In, F>, S): Routed<&'a In, I>,
    {
        Formation {
            changed: self.changed,
            strata: (
                Router {
                    f,
                    marker: PhantomData,
                },
                self.strata,
            ),
        }
    }
}

impl<S: Strata> Formation<S> {
    /// Feeds an input by value, or a router by reference.
    ///
    /// Feeding an input its current value changes nothing.
    pub fn feed<T, I>(&mut self, value: T)
    where
        S: Feed<T, I>,
    {
        self.strata.feed(&mut self.changed, value);
    }

    /// Reads the signal `T` without recomputing any memo.
    ///
    /// Inputs and memos are read by reference, while views are computed on
    /// the spot and returned by value.
    ///
    /// Inputs are always [`Fresh`](Reading::Fresh). A memo is
    /// [`Stale`](Reading::Stale) whenever a signal it depends on, directly or
    /// through other signals, changed since the last [`poll`](Self::poll).
    /// Signals declared since then count as changed, so memos over them are
    /// stale until the next poll. A view is stale whenever a memo it is
    /// computed from is.
    pub fn get<T, I>(&self) -> Reading<<S as Get<T, I>>::Ref<'_>>
    where
        S: Get<T, I>,
    {
        self.strata.get(&self.changed)
    }

    /// Recomputes the memos whose dependencies changed and reports every
    /// changed signal, inputs and views included, to `visitor`.
    ///
    /// A memo whose value did not change is neither reported nor propagated
    /// to its dependents. A view is reported and propagated whenever one of
    /// its dependencies changed, whether or not its own value did.
    pub fn poll<V>(&mut self, visitor: &mut V)
    where
        S: Report<V>,
    {
        if self.changed == S::Flags::default() {
            return;
        }
        self.strata.evaluate(&mut self.changed);
        self.strata
            .report(&core::mem::take(&mut self.changed), visitor);
    }
}
