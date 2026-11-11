#![no_std]

use core::hint::black_box;

use outcrop::Formation;
use outcrop::Slot;
use outcrop::Visit;
use outcrop::Write;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Register<const A: u16>(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Decoded<const A: u16>(pub [u16; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pair<const A: u16>(pub [u16; 16]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scaled(pub [u32; 8]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScaledView(pub [u32; 8]);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unrelated(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Flag(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Alarm(pub bool);

pub struct Frame {
    pub address: u16,
    pub value: u16,
}

#[derive(Default)]
pub struct Sum(pub u32);

impl<T> Visit<T> for Sum {
    fn visit(&mut self, value: &T) {
        black_box(value);
        self.0 += 1;
    }
}

fn decode<const A: u16>(r: &Register<A>) -> Decoded<A> {
    let mut out = [0; 16];
    for (i, o) in out.iter_mut().enumerate() {
        *o = r.0.rotate_left(i as u32) ^ A;
    }
    Decoded(out)
}

fn pair<const A: u16, const B: u16>(a: &Decoded<A>, b: &Decoded<B>) -> Pair<A> {
    let mut out = [0; 16];
    for (i, o) in out.iter_mut().enumerate() {
        *o = a.0[i].wrapping_add(b.0[15 - i]);
    }
    Pair(out)
}

fn flag(r: &Register<1>) -> Flag {
    Flag(r.0 & 0x8000 != 0)
}

fn alarm(f: &Flag, d: &Decoded<1>) -> Alarm {
    Alarm(f.0 && d.0[0] == 0)
}

fn scaled(r: &Register<0>) -> Scaled {
    let mut out = [0; 8];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u32::from(r.0) * (i as u32 + 1);
    }
    Scaled(out)
}

fn scaled_view(s: &Scaled) -> ScaledView {
    ScaledView(s.0.map(|v| v / 3))
}

fn summary(a: &Pair<0>, b: &Pair<2>, c: &Pair<4>, d: &Pair<6>, s: &ScaledView) -> Summary {
    let fold = |p: &[u16; 16]| p.iter().map(|v| u32::from(*v)).sum::<u32>();
    Summary(fold(&a.0) ^ fold(&b.0) ^ fold(&c.0) ^ fold(&d.0) ^ s.0[0])
}

macro_rules! builder {
    () => {
        Formation::new()
            .input(Unrelated(0))
            .input(Register::<0>(1))
            .input(Register::<1>(1))
            .input(Register::<2>(1))
            .input(Register::<3>(1))
            .input(Register::<4>(1))
            .input(Register::<5>(1))
            .input(Register::<6>(1))
            .input(Register::<7>(1))
            .route(|f: &Frame, sink| match f.address {
                0 => sink.feed(Register::<0>(f.value)),
                1 => sink.feed(Register::<1>(f.value)),
                2 => sink.feed(Register::<2>(f.value)),
                3 => sink.feed(Register::<3>(f.value)),
                4 => sink.feed(Register::<4>(f.value)),
                5 => sink.feed(Register::<5>(f.value)),
                6 => sink.feed(Register::<6>(f.value)),
                7 => sink.feed(Register::<7>(f.value)),
                _ => sink.feed(Unrelated(f.value)),
            })
            .memo(scaled)
            .view(decode::<0>)
            .view(decode::<1>)
            .view(decode::<2>)
            .view(decode::<3>)
            .view(decode::<4>)
            .view(decode::<5>)
            .view(decode::<6>)
            .view(decode::<7>)
            .view(pair::<0, 1>)
            .view(pair::<2, 3>)
            .view(pair::<4, 5>)
            .view(pair::<6, 7>)
            .view(scaled_view)
            .memo(summary)
            .view(flag)
            .memo(alarm)
    };
}

outcrop::strata! {
    struct Heavy {
        unrelated: Unrelated = Unrelated(0),
        r0: Register<0> = Register(1),
        r1: Register<1> = Register(1),
        r2: Register<2> = Register(1),
        r3: Register<3> = Register(1),
        r4: Register<4> = Register(1),
        r5: Register<5> = Register(1),
        r6: Register<6> = Register(1),
        r7: Register<7> = Register(1),
        Frame => |f, sink| match f.address {
            0 => sink.feed(Register::<0>(f.value)),
            1 => sink.feed(Register::<1>(f.value)),
            2 => sink.feed(Register::<2>(f.value)),
            3 => sink.feed(Register::<3>(f.value)),
            4 => sink.feed(Register::<4>(f.value)),
            5 => sink.feed(Register::<5>(f.value)),
            6 => sink.feed(Register::<6>(f.value)),
            7 => sink.feed(Register::<7>(f.value)),
            _ => sink.feed(Unrelated(f.value)),
        },
        scaled: Scaled = Register<0> => scaled,
        Decoded<0> = Register<0> => decode::<0>,
        Decoded<1> = Register<1> => decode::<1>,
        Decoded<2> = Register<2> => decode::<2>,
        Decoded<3> = Register<3> => decode::<3>,
        Decoded<4> = Register<4> => decode::<4>,
        Decoded<5> = Register<5> => decode::<5>,
        Decoded<6> = Register<6> => decode::<6>,
        Decoded<7> = Register<7> => decode::<7>,
        Pair<0> = (Decoded<0>, Decoded<1>) => pair::<0, 1>,
        Pair<2> = (Decoded<2>, Decoded<3>) => pair::<2, 3>,
        Pair<4> = (Decoded<4>, Decoded<5>) => pair::<4, 5>,
        Pair<6> = (Decoded<6>, Decoded<7>) => pair::<6, 7>,
        ScaledView = Scaled => scaled_view,
        summary: Summary = (Pair<0>, Pair<2>, Pair<4>, Pair<6>, ScaledView) => summary,
        Flag = Register<1> => flag,
        alarm: Alarm = (Flag, Decoded<1>) => alarm,
    }
}

macro_rules! scenarios {
    ($make:expr, $setup:ident, $bottom:ident, $unrelated:ident, $top:ident, $flat:ident, $route:ident, $get:ident) => {
        /// Builds the formation and polls it `rounds` times without feeding.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $setup(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            for _ in 0..=black_box(rounds) {
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0
        }

        /// Feeds the lowest register, which every view sits above.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $bottom(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            formation.poll(&mut sum);
            for i in 0..black_box(rounds) {
                formation.feed(Register::<0>(black_box(i as u16)));
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0
        }

        /// Feeds an input no view depends on.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $unrelated(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            formation.poll(&mut sum);
            for i in 0..black_box(rounds) {
                formation.feed(Unrelated(black_box(i as u16)));
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0
        }

        /// Feeds the highest register, which a single view chain depends on.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $top(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            formation.poll(&mut sum);
            for i in 0..black_box(rounds) {
                formation.feed(Register::<7>(black_box(i as u16)));
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0
        }

        /// Feeds a register whose `Flag` view never changes value.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $flat(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            formation.poll(&mut sum);
            for i in 0..black_box(rounds) {
                formation.feed(Register::<1>(black_box(i as u16 + 2)));
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0
        }

        /// Routes frames that write every register and the unrelated input.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $route(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            formation.poll(&mut sum);
            for i in 0..black_box(rounds) {
                for address in 0..9 {
                    formation.feed(&Frame {
                        address: black_box(address),
                        value: black_box(i as u16),
                    });
                }
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0
        }

        /// Feeds the lowest register and reads two memos before polling.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $get(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            formation.poll(&mut sum);
            let mut stale = 0;
            for i in 0..black_box(rounds) {
                formation.feed(Register::<0>(black_box(i as u16)));
                stale += u32::from(formation.get::<Summary, _>().is_stale());
                stale += u32::from(formation.get::<Alarm, _>().is_stale());
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0 + stale
        }
    };
}

scenarios!(
    builder!(),
    builder_idle,
    builder_feed_bottom,
    builder_feed_unrelated,
    builder_feed_top,
    builder_feed_flat,
    builder_route,
    builder_get
);

scenarios!(
    Formation::from(Heavy::default()),
    macro_idle,
    macro_feed_bottom,
    macro_feed_unrelated,
    macro_feed_top,
    macro_feed_flat,
    macro_route,
    macro_get
);

/// A word of a bank of 100 registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Word<const A: usize>(pub u16);

impl<const A: usize> Slot for Word<A> {
    type Item = u16;
    const INDEX: usize = A;

    fn read(item: &u16) -> Self {
        Self(*item)
    }
}

fn spread(a: &Word<3>, b: &Word<50>, c: &Word<97>) -> Summary {
    Summary(u32::from(a.0) ^ u32::from(b.0) << 8 ^ u32::from(c.0) << 16)
}

macro_rules! banked {
    () => {
        Formation::new()
            .bank::<13, _, _, _>([0_u16; 100])
            .view(|w: &Word<3>| Flag(w.0 != 0))
            .view(|w: &Word<50>| Unrelated(w.0))
            .memo(spread)
    };
}

outcrop::strata! {
    struct Banked {
        #[strata(bank)]
        words: [u16; 100] = [0; 100],
        Flag = Word<3> => |w| Flag(w.0 != 0),
        Unrelated = Word<50> => |w| Unrelated(w.0),
        spread: Summary = (Word<3>, Word<50>, Word<97>) => spread,
    }
}

outcrop::strata! {
    struct Words {
        #[strata(bank)]
        words: [u16; 100] = [0; 100],
    }
}

macro_rules! nested {
    () => {
        Formation::new()
            .nest(Words::default())
            .view(|w: &Word<3>| Flag(w.0 != 0))
            .view(|w: &Word<50>| Unrelated(w.0))
            .memo(spread)
    };
}

macro_rules! bank_scenarios {
    ($make:expr, $one:ident, $all:ident) => {
        /// Writes one word of the bank, with no signal on it, every round.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $one(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            formation.poll(&mut sum);
            for i in 0..black_box(rounds) {
                formation.feed(Write::new(black_box(70), &[black_box(i as u16 + 1)]));
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0
        }

        /// Writes every word of the bank with a new value every round.
        #[unsafe(no_mangle)]
        #[inline(never)]
        pub extern "C" fn $all(rounds: u32) -> u32 {
            let mut formation = black_box($make);
            let mut sum = Sum::default();
            formation.poll(&mut sum);
            for i in 0..black_box(rounds) {
                let words = [black_box(i as u16 + 1); 100];
                formation.feed(Write::new(0, &words));
                formation.poll(&mut sum);
            }
            black_box(&formation);
            sum.0
        }
    };
}

bank_scenarios!(banked!(), builder_bank_one, builder_bank_all);
bank_scenarios!(Formation::from(Banked::default()), macro_bank_one, macro_bank_all);
bank_scenarios!(nested!(), nest_bank_one, nest_bank_all);

/// Every scenario, by name.
pub const SCENARIOS: [(&str, extern "C" fn(u32) -> u32); 20] = [
    ("builder_idle", builder_idle),
    ("builder_feed_bottom", builder_feed_bottom),
    ("builder_feed_unrelated", builder_feed_unrelated),
    ("builder_feed_top", builder_feed_top),
    ("builder_feed_flat", builder_feed_flat),
    ("builder_route", builder_route),
    ("builder_get", builder_get),
    ("macro_idle", macro_idle),
    ("macro_feed_bottom", macro_feed_bottom),
    ("macro_feed_unrelated", macro_feed_unrelated),
    ("macro_feed_top", macro_feed_top),
    ("macro_feed_flat", macro_feed_flat),
    ("macro_route", macro_route),
    ("macro_get", macro_get),
    ("builder_bank_one", builder_bank_one),
    ("builder_bank_all", builder_bank_all),
    ("macro_bank_one", macro_bank_one),
    ("macro_bank_all", macro_bank_all),
    ("nest_bank_one", nest_bank_one),
    ("nest_bank_all", nest_bank_all),
];

/// Formation sizes in bytes, builder then macro.
pub fn sizes() -> (usize, usize) {
    (
        size_of_val(&builder!()),
        size_of_val(&Formation::from(Heavy::default())),
    )
}

#[cfg(target_arch = "wasm32")]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo<'_>) -> ! {
    core::arch::wasm32::unreachable()
}
