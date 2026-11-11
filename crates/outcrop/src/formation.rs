use core::marker::PhantomData;

#[cfg(feature = "macros")]
use crate::__private::Nestable;
#[cfg(feature = "macros")]
use crate::nested::Nested;
use crate::plumbing::Layer;
use crate::plumbing::Sound;
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
/// Depending on a signal the formation does not contain, or feeding a memo or
/// a view, is a compile-time error. Every signal has its own type, so
/// declaring or routing a type twice is rejected too.
///
/// Each layer is checked against every layer beneath it, about two levels of
/// trait resolution per layer, so a formation of 64 layers or more exceeds
/// the compiler's default recursion limit of 128, and one of 63 can when its
/// top layer is a memo or view. The compiler reports E0275, "overflow
/// evaluating the requirement".
/// Raise the limit in the crate that builds the formation, with
/// `#![recursion_limit = "256"]`. A nested `strata!` struct counts as one
/// layer; the struct itself has limits of its own, listed on `strata!`.
///
/// Two formations are equal, and hash alike, when their signals hold equal
/// values and the same changes await the next [`poll`](Self::poll). Until
/// then a memo may still hold its value from before a feed, so the pending
/// changes are part of the state: poll both formations before comparing them
/// by value. Only formations of strata that implement these traits, such as a
/// `strata!` struct deriving them, can be compared.
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
    ///
    /// Inputs and memos are compared to tell whether they changed, so they
    /// must implement `Eq`: a value unequal to itself, such as a floating-point
    /// NaN, would be reported as changed every time. Wrap such values in a type
    /// whose `Eq` compares them by their bits, for instance.
    ///
    /// Every signal of a formation has a distinct type, and an input cannot
    /// share its type with a router beneath it either, since feeding that type
    /// could not tell them apart. The indices that tell layers apart cannot
    /// be named, so the check cannot be bypassed by spelling one out.
    #[must_use]
    pub fn input<T: Eq, I>(self, initial: T) -> Deposited<Input<T>, S>
    where
        (Input<T>, S): Sound<I>,
    {
        self.deposit(Input::new(initial))
    }

    /// Deposits a bank: an array of inputs with a change bit per element,
    /// each read as the [`Slot`](crate::Slot) whose index points at it.
    ///
    /// A bank is fed a [`Write`](crate::Write), which marks only the elements
    /// whose value changed, so a signal depending on a slot is recomputed and
    /// reported only when its element changed. [`poll`](Self::poll) reports
    /// every changed element as an [`Element`](crate::Element), and the next
    /// one reports them all.
    ///
    /// The change bits are packed in `BYTES` bytes, which must be
    /// `N.div_ceil(8)` and is given explicitly, since stable Rust cannot
    /// compute it from `N`. The whole bank costs one level of the recursion
    /// limit, however many elements it holds.
    ///
    /// Those bytes are the bank's alone: the bits left in the byte beneath it
    /// and in its own last byte go unused, and the next signal starts a new
    /// byte. Tracking how many bits a bank takes would need arithmetic on
    /// const generics, which stable Rust lacks. So an input, a bank of four
    /// and another input take three bytes of change bits where one would do,
    /// at most two bytes lost per bank. `strata!` packs its banks with the
    /// signals around them.
    ///
    /// ```
    /// use outcrop::{Formation, Slot, Write};
    ///
    /// #[derive(Debug, PartialEq, Eq)]
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
    /// #[derive(Debug, PartialEq, Eq)]
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
    /// A slot is any type implementing [`Slot`](crate::Slot) for the bank's
    /// item type, so a bank cannot list its slots when it is deposited: a slot
    /// of the same type as another signal is only rejected where it is read.
    ///
    /// Two banks of the same item type are rejected, since a `Write` could
    /// not tell them apart, and so is a byte count other than `N.div_ceil(8)`:
    ///
    /// ```compile_fail,E0080
    /// # use outcrop::Formation;
    /// let formation = Formation::new().bank::<1, _, _, _>([0_u16; 9]);
    /// ```
    ///
    /// Or a slot past the end of the bank:
    ///
    /// ```compile_fail,E0080
    /// # use outcrop::{Formation, Slot};
    /// # #[derive(PartialEq, Eq)]
    /// # struct Register<const ADDRESS: usize>(u16);
    /// # impl<const ADDRESS: usize> Slot for Register<ADDRESS> {
    /// #     type Item = u16;
    /// #     const INDEX: usize = ADDRESS;
    /// #     fn read(item: &u16) -> Self { Self(*item) }
    /// # }
    /// # #[derive(PartialEq, Eq)]
    /// # struct Celsius(u16);
    /// let mut formation = Formation::new()
    ///     .bank::<1, _, _, _>([0_u16; 4])
    ///     .memo(|r: &Register<4>| Celsius(r.0 / 10));
    /// formation.poll(&mut ());
    /// ```
    ///
    /// Both checks are evaluated when the code is compiled to machine code, so
    /// `cargo build` and `cargo test` report them but `cargo check` does not,
    /// and a formation that is never used is not checked.
    #[must_use]
    pub fn bank<const BYTES: usize, T: Eq + Clone, const N: usize, I>(
        self,
        initial: [T; N],
    ) -> Deposited<Bank<T, N, BYTES>, S>
    where
        (Bank<T, N, BYTES>, S): Sound<I>,
    {
        const {
            assert!(
                BYTES == N.div_ceil(8),
                "a bank of `N` elements packs its change bits in `N.div_ceil(8)` bytes"
            );
        };
        self.deposit(Bank::new(initial))
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
        F::Output: Eq,
        (Memo<F, M, F::Output>, S): Sound<I>,
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
    /// Its value is never compared, so it needs no `Eq`:
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
        (View<F, M, F::Output>, S): Sound<I>,
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
    /// A router can only feed what lies beneath it.
    #[must_use]
    pub fn route<In: ?Sized, F, I>(self, f: F) -> Deposited<Router<In, F>, S>
    where
        F: FnMut(&In, &mut Sink<'_, S>),
        (Router<In, F>, S): Sound<I>,
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
    /// like any other, so each must be distinct from the signals of the
    /// formation, and the types it is fed from those the formation is fed.
    /// The struct's bank slots are the exception: a slot clashing with a
    /// signal of the formation is only rejected where it is read.
    ///
    /// As with a [`bank`](Self::bank), the struct's change bits start on a
    /// byte of their own and the next signal starts another, so up to two
    /// bytes of bits go unused.
    ///
    /// ```
    /// use outcrop::Formation;
    ///
    /// #[derive(Debug, PartialEq, Eq)]
    /// struct Kelvin(u16);
    /// #[derive(Debug, PartialEq, Eq)]
    /// struct Celsius(i32);
    /// #[derive(Debug, PartialEq, Eq)]
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
    /// So nesting a struct that declares a type the formation already holds
    /// is rejected, as is nesting the same struct twice.
    #[cfg(feature = "macros")]
    #[must_use]
    pub fn nest<N, I>(self, strata: N) -> Deposited<Nested<N>, S>
    where
        N: Strata + Nestable<S, I>,
        Nested<N>: Layer<S>,
    {
        self.deposit(Nested(strata))
    }

    /// Deposits `layer` on top of the strata, with its bits marked as
    /// changed, once it holds no type the strata already hold.
    fn deposit<L: Layer<S>, I>(self, layer: L) -> Deposited<L, S>
    where
        (L, S): Sound<I>,
    {
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
    /// #[derive(Debug, PartialEq, Eq)]
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
    /// Inputs, bank slots and views computed from them alone are always
    /// [`Fresh`](Reading::Fresh). Memos, and views computed from a memo, are
    /// [`Stale`](Reading::Stale) whenever anything in the formation changed
    /// since the last [`poll`](Self::poll), related to them or not: telling
    /// the two apart would cost a walk of their dependencies on every read.
    /// Signals declared since then count as changed.
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
    ///
    /// Under `panic = "unwind"`, each signal is unmarked just before it is
    /// visited: if `visitor` panics and the panic is caught, the signals it had
    /// not reached yet are reported by the next poll; the one it panicked on is
    /// not, so a visitor that always panics on a value cannot block the others.
    pub fn poll<V>(&mut self, visitor: &mut V)
    where
        S: Report<V>,
    {
        if self.changed == S::Flags::default() {
            return;
        }
        self.strata.evaluate(&mut self.changed);
        self.strata.report(&mut self.changed, visitor);
        self.changed = S::Flags::default();
    }
}
