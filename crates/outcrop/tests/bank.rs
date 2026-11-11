//! Banks: arrays of inputs tracking a change bit per element.
#![expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "derived signals receive their dependencies by reference"
)]

mod common;

use common::Register;
use common::poll;
use outcrop::Element;
use outcrop::Formation;
use outcrop::Reading;
use outcrop::Visit;
use outcrop::Write;

#[derive(Debug, PartialEq, Eq)]
struct Celsius(u16);

#[derive(Debug, PartialEq, Eq)]
struct Total(u32);

#[derive(Debug, PartialEq, Eq)]
struct Label(&'static str);

fn celsius(r: &Register<1>) -> Celsius {
    Celsius(r.0 / 10)
}

fn total(low: &Register<2>, high: &Register<3>) -> Total {
    Total(u32::from(high.0) << 16 | u32::from(low.0))
}

#[test]
fn first_poll_reports_every_element_and_signal() {
    let mut formation = Formation::new()
        .bank::<1, _, _, _>([1_u16, 2])
        .memo(celsius);

    assert_eq!(
        poll(&mut formation),
        [
            "Element { index: 0, value: 1 }",
            "Element { index: 1, value: 2 }",
            "Celsius(0)",
        ]
    );
}

#[test]
fn writes_report_only_the_elements_that_changed() {
    let mut formation = Formation::new().bank::<1, _, _, _>([0_u16; 4]);
    poll(&mut formation);

    formation.feed(Write::new(1, &[0, 7, 0]));

    assert_eq!(poll(&mut formation), ["Element { index: 2, value: 7 }"]);
}

#[test]
fn signals_are_recomputed_only_when_their_elements_changed() {
    let mut formation = Formation::new()
        .bank::<1, _, _, _>([0_u16; 4])
        .memo(celsius)
        .view(total);
    poll(&mut formation);

    formation.feed(Write::new(0, &[9]));

    assert_eq!(poll(&mut formation), ["Element { index: 0, value: 9 }"]);

    formation.feed(Write::new(1, &[215, 1]));

    assert_eq!(
        poll(&mut formation),
        [
            "Element { index: 1, value: 215 }",
            "Element { index: 2, value: 1 }",
            "Celsius(21)",
            "Total(1)",
        ]
    );
}

#[test]
fn writes_past_the_end_of_the_bank_are_ignored() {
    let mut formation = Formation::new().bank::<1, _, _, _>([0_u16; 2]);
    poll(&mut formation);

    formation.feed(Write::new(1, &[5, 6, 7]));
    formation.feed(Write::new(9, &[1]));

    assert_eq!(poll(&mut formation), ["Element { index: 1, value: 5 }"]);
}

#[test]
fn slots_are_read_by_type() {
    let mut formation = Formation::new()
        .bank::<1, _, _, _>([0_u16; 2])
        .memo(celsius);
    formation.feed(Write::new(1, &[300]));

    assert_eq!(
        formation.get::<Register<1>, _>(),
        Reading::Fresh(Register(300))
    );
    assert_eq!(formation.get::<Celsius, _>(), Reading::Stale(&Celsius(0)));
}

#[test]
fn signals_around_a_bank_keep_their_own_bits() {
    let mut formation = Formation::new()
        .input(Label("a"))
        .bank::<2, _, _, _>([0_u16; 11])
        .memo(celsius)
        .input(Total(0));
    poll(&mut formation);

    formation.feed(Label("b"));
    formation.feed(Total(1));
    formation.feed(Write::new(10, &[4]));

    assert_eq!(
        poll(&mut formation),
        [
            "Label(\"b\")",
            "Element { index: 10, value: 4 }",
            "Total(1)",
        ]
    );
}

#[test]
fn routers_feed_banks_beneath_them() {
    let mut formation = Formation::new()
        .bank::<1, _, _, _>([0_u16; 2])
        .memo(celsius)
        .route(|bytes: &[u8], sink| {
            let words = bytes.iter().copied().map(u16::from).collect::<Vec<_>>();
            sink.feed(Write::new(0, &words));
        });
    poll(&mut formation);

    formation.feed(&[0_u8, 200][..]);

    assert_eq!(
        poll(&mut formation),
        ["Element { index: 1, value: 200 }", "Celsius(20)"]
    );
}

#[test]
fn elements_carry_their_index() {
    struct Indices(Vec<usize>);

    impl Visit<Element<'_, u16>> for Indices {
        fn visit(&mut self, element: &Element<'_, u16>) {
            self.0.push(element.index);
        }
    }

    let mut formation = Formation::new().bank::<1, _, _, _>([0_u16; 3]);
    let mut indices = Indices(Vec::new());
    formation.poll(&mut indices);

    assert_eq!(indices.0, [0, 1, 2]);
}

#[test]
fn builder_banks_pack_a_change_bit_per_element() {
    let formation = Formation::new().bank::<2, _, _, _>([0_u8; 11]);

    assert_eq!(size_of_val(&formation), 1 + 2 + 11);
}

#[cfg(feature = "macros")]
mod declared {
    use super::*;

    #[derive(Debug, PartialEq, Eq)]
    struct Missing;

    outcrop::strata! {
        struct Probe {
            label: Label = Label("a"),
            #[cfg(any())]
            #[strata(bank)]
            missing: [Missing; 3] = [Missing, Missing, Missing],
            #[strata(bank)]
            registers: [u16; 11] = [0; 11],
            celsius: Celsius = Register<1> => celsius,
            Total = (Register<2>, Register<3>) => total,
            [u8] => |bytes, sink| {
                let words = bytes.iter().copied().map(u16::from).collect::<Vec<_>>();
                sink.feed(Write::new(0, &words));
            },
        }
    }

    #[test]
    fn declared_banks_report_like_built_ones() {
        let mut built = Formation::new()
            .input(Label("a"))
            .bank::<2, _, _, _>([0_u16; 11])
            .memo(celsius)
            .view(total)
            .route(|bytes: &[u8], sink| {
                let words = bytes.iter().copied().map(u16::from).collect::<Vec<_>>();
                sink.feed(Write::new(0, &words));
            });
        let mut declared = Formation::from(Probe::default());

        assert_eq!(poll(&mut declared), poll(&mut built));

        built.feed(Write::new(2, &[1, 0, 9]));
        declared.feed(Write::new(2, &[1, 0, 9]));
        assert_eq!(poll(&mut declared), poll(&mut built));

        built.feed(&[0_u8, 250][..]);
        declared.feed(&[0_u8, 250][..]);
        assert_eq!(poll(&mut declared), poll(&mut built));

        built.feed(Label("b"));
        declared.feed(Label("b"));
        built.feed(Write::new(10, &[3]));
        declared.feed(Write::new(10, &[3]));
        let expected = ["Label(\"b\")", "Element { index: 10, value: 3 }"];
        assert_eq!(poll(&mut declared), expected);
        assert_eq!(poll(&mut built), expected);
    }

    #[test]
    fn declared_slots_are_read_by_type() {
        let mut formation = Formation::from(Probe::default());
        formation.feed(Write::new(1, &[300]));

        assert_eq!(
            formation.get::<Register<1>, _>(),
            Reading::Fresh(Register(300))
        );
        assert_eq!(formation.get::<Celsius, _>(), Reading::Stale(&Celsius(0)));
    }
}

/// Panics on the bank element at its index.
struct PanicAt(usize);

impl Visit<Element<'_, u16>> for PanicAt {
    fn visit(&mut self, element: &Element<'_, u16>) {
        assert_ne!(element.index, self.0, "visitor panics");
    }
}

#[test]
fn a_panicking_visitor_leaves_the_elements_after_it_to_the_next_poll() {
    let mut formation = Formation::new().bank::<2, _, _, _>([0_u16; 10]);
    poll(&mut formation);
    formation.feed(Write::new(1, &[1, 2, 3]));
    formation.feed(Write::new(9, &[4]));

    let panicked = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        formation.poll(&mut PanicAt(2));
    }));

    assert!(panicked.is_err());
    assert_eq!(
        poll(&mut formation),
        [
            "Element { index: 3, value: 3 }",
            "Element { index: 9, value: 4 }"
        ]
    );
}
