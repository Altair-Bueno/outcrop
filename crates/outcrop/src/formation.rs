use core::marker::PhantomData;

use crate::bank::Write;
#[cfg(feature = "macros")]
use crate::nested::Nested;
use crate::plumbing::Distinct;
use crate::plumbing::Layer;
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
        self.deposit(Input(initial))
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
    /// The change bits are packed in `BYTES` bytes, which must be
    /// `N.div_ceil(8)` and is given explicitly, since stable Rust cannot
    /// compute it from `N`. The whole bank costs one level of the recursion
    /// limit, however many elements it holds.
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
    ///     .bank::<1, _, _, _>([0_u16; 4])
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
    /// let formation = Formation::new()
    ///     .bank::<1, _, _, _>([0_u16; 2])
    ///     .bank::<1, _, _, _>([0_u16; 3]);
    /// ```
    ///
    /// And so is a byte count other than `N.div_ceil(8)`:
    ///
    /// ```compile_fail,E0080
    /// # use outcrop::Formation;
    /// let formation = Formation::new().bank::<1, _, _, _>([0_u16; 9]);
    /// ```
    #[must_use]
    pub fn bank<const BYTES: usize, T: PartialEq + Clone, const N: usize, I>(
        self,
        initial: [T; N],
    ) -> Deposited<Bank<T, N, BYTES>, S>
    where
        for<'w> (Bank<T, N, BYTES>, S): Routed<Write<'w, T>, I>,
    {
        const {
            assert!(
                BYTES == N.div_ceil(8),
                "a bank of `N` elements packs its change bits in `N.div_ceil(8)` bytes"
            );
        };
        self.deposit(Bank(initial))
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
        self.deposit(Memo {
            f,
            value,
            marker: PhantomData,
        })
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
        self.deposit(View {
            f,
            marker: PhantomData,
        })
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
        self.deposit(Router {
            f,
            marker: PhantomData,
        })
    }

    /// Deposits a struct declared with `strata!` as one layer, whose signals
    /// the signals above can depend on, and that the formation and the
    /// routers above can feed.
    ///
    /// Every signal of the struct is reported by the next
    /// [`poll`](Self::poll). The struct depends only on its own signals,
    /// never on those beneath it, and counts as one signal towards the
    /// recursion limit, however many it holds. Its signals are found by type
    /// like any other, so nesting the same struct twice, or a struct holding
    /// a type already declared, makes looking that type up ambiguous.
    ///
    /// ```
    /// use outcrop::Formation;
    ///
    /// #[derive(Debug, PartialEq)]
    /// struct Kelvin(u16);
    /// #[derive(Debug, PartialEq)]
    /// struct Celsius(i32);
    /// #[derive(Debug, PartialEq)]
    /// struct Freezing(bool);
    ///
    /// outcrop::strata! {
    ///     struct Probe {
    ///         kelvin: Kelvin = Kelvin(293),
    ///         Celsius = Kelvin => |k| Celsius(i32::from(k.0) - 273),
    ///     }
    /// }
    ///
    /// let mut formation = Formation::new()
    ///     .nest(Probe::default())
    ///     .memo(|c: &Celsius| Freezing(c.0 <= 0));
    ///
    /// formation.feed(Kelvin(270));
    /// formation.poll(&mut ());
    /// assert_eq!(*formation.get::<Freezing, _>().into_inner(), Freezing(true));
    /// ```
    ///
    /// A type declared both in the struct and in the formation cannot be
    /// depended on:
    ///
    /// ```compile_fail
    /// # use outcrop::Formation;
    /// #[derive(PartialEq)]
    /// struct Kelvin(u16);
    /// #[derive(PartialEq)]
    /// struct Hot(bool);
    ///
    /// outcrop::strata! {
    ///     struct Probe {
    ///         kelvin: Kelvin = Kelvin(293),
    ///     }
    /// }
    ///
    /// let formation = Formation::new()
    ///     .input(Kelvin(0))
    ///     .nest(Probe::default())
    ///     .memo(|k: &Kelvin| Hot(k.0 > 300));
    /// ```
    #[cfg(feature = "macros")]
    #[must_use]
    pub fn nest<N: Strata>(self, strata: N) -> Deposited<Nested<N>, S>
    where
        Nested<N>: Layer<S>,
    {
        self.deposit(Nested(strata))
    }

    /// Deposits `layer` on top of the strata, with its bits marked as
    /// changed.
    fn deposit<L: Layer<S>>(self, layer: L) -> Deposited<L, S> {
        Formation {
            changed: L::deposit(self.changed),
            strata: (layer, self.strata),
        }
    }
}

impl<S: Strata> Formation<S> {
    /// Feeds an input by value, or a router by reference.
    ///
    /// Feeding an input its current value changes nothing. Feeding it a
    /// different value marks it as changed until the next
    /// [`poll`](Self::poll), even if a later feed restores the value it had
    /// then: the input is compared with its current value, not with the last
    /// one reported.
    ///
    /// ```
    /// # use outcrop::{Formation, Visit};
    /// #[derive(Debug, PartialEq)]
    /// struct A(u8);
    ///
    /// struct Count(usize);
    ///
    /// impl<T> Visit<T> for Count {
    ///     fn visit(&mut self, _: &T) {
    ///         self.0 += 1;
    ///     }
    /// }
    ///
    /// let mut formation = Formation::new().input(A(0));
    /// formation.poll(&mut ());
    /// formation.feed(A(1));
    /// formation.feed(A(0));
    ///
    /// let mut count = Count(0);
    /// formation.poll(&mut count);
    /// assert_eq!(count.0, 1); // A(0), written over since the last poll
    /// ```
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
    /// An input is reported if it was [fed](Self::feed) a different value
    /// since the last poll, even if a later feed restored it. A memo whose
    /// value did not change is neither reported nor propagated
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
