//! Behaviour of formations built from inputs and derived signals.

use core::cell::Cell;
use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Report;
use outcrop::Visit;

#[derive(Debug, PartialEq)]
struct Temperature(i16);

#[derive(Debug, PartialEq)]
struct AntiFreeze(bool);

#[derive(Debug, PartialEq)]
struct Setpoint(i16);

#[derive(Debug, PartialEq)]
struct Heating(bool);

#[derive(Default)]
struct Recorder(Vec<String>);

impl<T: Debug> Visit<T> for Recorder {
    fn visit(&mut self, value: &T) {
        self.0.push(format!("{value:?}"));
    }
}

fn poll<S>(formation: &mut Formation<S>) -> Vec<String>
where
    S: Report<Recorder>,
{
    let mut recorder = Recorder::default();
    formation.poll(&mut recorder);
    recorder.0
}

fn heating(t: &Temperature, s: &Setpoint) -> Heating {
    Heating(t.0 < s.0)
}

#[test]
fn first_poll_reports_every_initial_value() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .input(Setpoint(18))
        .with(heating);

    assert_eq!(
        poll(&mut formation),
        ["Temperature(20)", "Setpoint(18)", "Heating(false)"]
    );
}

#[test]
fn default_formation_accepts_strata_like_a_new_one() {
    let mut formation = Formation::default().input(Temperature(20));

    assert_eq!(poll(&mut formation), ["Temperature(20)"]);
}

#[test]
fn poll_without_feeds_reports_nothing() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .with(|t: &Temperature| AntiFreeze(t.0 < 10));
    poll(&mut formation);

    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn feeding_an_input_reports_it_and_its_changed_dependents() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .input(Setpoint(18))
        .with(heating);
    poll(&mut formation);

    formation.feed(Temperature(15));

    assert_eq!(poll(&mut formation), ["Temperature(15)", "Heating(true)"]);
}

#[test]
fn feeding_the_current_value_reports_nothing() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .with(|t: &Temperature| AntiFreeze(t.0 < 10));
    poll(&mut formation);

    formation.feed(Temperature(20));

    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn unchanged_derived_value_stops_propagation() {
    let downstream = Cell::new(0);
    let mut formation = Formation::new()
        .input(Temperature(20))
        .with(|t: &Temperature| AntiFreeze(t.0 < 10))
        .with(|a: &AntiFreeze| {
            downstream.set(downstream.get() + 1);
            Heating(a.0)
        });
    poll(&mut formation);
    let before = downstream.get();

    formation.feed(Temperature(15));

    assert_eq!(poll(&mut formation), ["Temperature(15)"]);
    assert_eq!(downstream.get(), before);
}

#[test]
fn signals_without_changed_dependencies_are_not_recomputed() {
    let calls = Cell::new(0);
    let mut formation = Formation::new()
        .input(Temperature(20))
        .input(Setpoint(18))
        .with(|s: &Setpoint| {
            calls.set(calls.get() + 1);
            Heating(s.0 > 0)
        });
    poll(&mut formation);
    let before = calls.get();

    formation.feed(Temperature(0));
    poll(&mut formation);

    assert_eq!(calls.get(), before);
}

#[test]
fn several_changed_dependencies_recompute_once() {
    let calls = Cell::new(0);
    let mut formation = Formation::new()
        .input(Temperature(20))
        .input(Setpoint(18))
        .with(|t: &Temperature, s: &Setpoint| {
            calls.set(calls.get() + 1);
            heating(t, s)
        });
    poll(&mut formation);
    let before = calls.get();

    formation.feed(Temperature(10));
    formation.feed(Setpoint(12));
    poll(&mut formation);

    assert_eq!(calls.get(), before + 1);
}

#[test]
fn derived_signals_see_values_derived_in_the_same_poll() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .with(|t: &Temperature| AntiFreeze(t.0 < 10))
        .with(|a: &AntiFreeze| Heating(a.0));
    poll(&mut formation);

    formation.feed(Temperature(0));

    assert_eq!(
        poll(&mut formation),
        ["Temperature(0)", "AntiFreeze(true)", "Heating(true)"]
    );
}

#[test]
fn derived_signals_accept_up_to_eight_dependencies() {
    #[derive(Debug, PartialEq)]
    struct Bit<const I: u8>(bool);
    #[derive(Debug, PartialEq)]
    struct Byte(u8);

    let mut formation = Formation::new()
        .input(Bit::<0>(false))
        .input(Bit::<1>(false))
        .input(Bit::<2>(false))
        .input(Bit::<3>(false))
        .input(Bit::<4>(false))
        .input(Bit::<5>(false))
        .input(Bit::<6>(false))
        .input(Bit::<7>(false))
        .with(
            |b0: &Bit<0>,
             b1: &Bit<1>,
             b2: &Bit<2>,
             b3: &Bit<3>,
             b4: &Bit<4>,
             b5: &Bit<5>,
             b6: &Bit<6>,
             b7: &Bit<7>| {
                Byte(
                    [b0.0, b1.0, b2.0, b3.0, b4.0, b5.0, b6.0, b7.0]
                        .into_iter()
                        .rev()
                        .fold(0, |byte, bit| byte << 1 | u8::from(bit)),
                )
            },
        );
    poll(&mut formation);

    formation.feed(Bit::<0>(true));
    formation.feed(Bit::<7>(true));

    assert_eq!(
        poll(&mut formation),
        ["Bit(true)", "Bit(true)", "Byte(129)"]
    );
}
