//! Behaviour of formations built from inputs and derived signals.

use core::cell::Cell;
use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Reading;
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
        .memo(heating);

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
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10));
    poll(&mut formation);

    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn feeding_an_input_reports_it_and_its_changed_dependents() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .input(Setpoint(18))
        .memo(heating);
    poll(&mut formation);

    formation.feed(Temperature(15));

    assert_eq!(poll(&mut formation), ["Temperature(15)", "Heating(true)"]);
}

#[test]
fn feeding_the_current_value_reports_nothing() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10));
    poll(&mut formation);

    formation.feed(Temperature(20));

    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn unchanged_derived_value_stops_propagation() {
    let downstream = Cell::new(0);
    let mut formation = Formation::new()
        .input(Temperature(20))
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10))
        .memo(|a: &AntiFreeze| {
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
        .memo(|s: &Setpoint| {
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
        .memo(|t: &Temperature, s: &Setpoint| {
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
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10))
        .memo(|a: &AntiFreeze| Heating(a.0));
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
        .memo(
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

struct Count(usize);

impl<T> Visit<T> for Count {
    fn visit(&mut self, _: &T) {
        self.0 += 1;
    }
}

#[test]
fn unit_discards_reported_signals() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .input(Setpoint(18))
        .memo(heating);

    formation.poll(&mut ());

    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

fn settle<S: Report<V>, V>(formation: &mut Formation<S>, mut visitor: V) {
    formation.poll(&mut visitor);
}

#[test]
fn references_forward_to_their_visitor() {
    let mut formation = Formation::new().input(Temperature(20));
    let mut count = Count(0);

    settle(&mut formation, &mut count);

    assert_eq!(count.0, 1);
}

#[test]
fn tuples_fan_out_in_order() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10));
    let mut recorder = Recorder::default();
    let mut count = Count(0);

    formation.poll(&mut (&mut recorder, &mut count, ()));

    assert_eq!(recorder.0, ["Temperature(20)", "AntiFreeze(false)"]);
    assert_eq!(count.0, 2);
}

#[test]
fn the_empty_formation_is_nameable() {
    let mut formation: Formation<()> = Formation::from(());

    formation.poll(&mut Count(0));
    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[derive(Debug, PartialEq)]
struct Alarm(bool);

#[test]
fn memos_declared_after_a_poll_over_settled_signals_are_fresh() {
    let mut formation = Formation::new().input(Temperature(20));
    poll(&mut formation);
    let mut formation = formation.memo(|t: &Temperature| AntiFreeze(t.0 < 10));

    assert_eq!(
        formation.get::<AntiFreeze, _>(),
        Reading::Fresh(&AntiFreeze(false))
    );
    assert_eq!(poll(&mut formation), ["AntiFreeze(false)"]);
}

#[test]
fn derived_reads_are_stale_until_the_first_poll() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10));

    assert_eq!(
        formation.get::<Temperature, _>(),
        Reading::Fresh(&Temperature(20))
    );
    assert_eq!(
        formation.get::<AntiFreeze, _>(),
        Reading::Stale(&AntiFreeze(false))
    );

    poll(&mut formation);

    assert_eq!(
        formation.get::<AntiFreeze, _>(),
        Reading::Fresh(&AntiFreeze(false))
    );
}

#[test]
fn feeding_marks_only_dependents_stale_until_the_next_poll() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .input(Setpoint(18))
        .memo(heating)
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10));
    poll(&mut formation);

    formation.feed(Setpoint(25));

    assert_eq!(
        formation.get::<Setpoint, _>(),
        Reading::Fresh(&Setpoint(25))
    );
    assert_eq!(
        formation.get::<Heating, _>(),
        Reading::Stale(&Heating(false))
    );
    assert_eq!(
        formation.get::<AntiFreeze, _>(),
        Reading::Fresh(&AntiFreeze(false))
    );

    poll(&mut formation);

    assert_eq!(
        formation.get::<Heating, _>(),
        Reading::Fresh(&Heating(true))
    );
}

#[test]
fn staleness_follows_derived_dependencies() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .input(Setpoint(18))
        .memo(heating)
        .memo(|h: &Heating| Alarm(h.0));
    poll(&mut formation);

    formation.feed(Temperature(10));

    assert_eq!(formation.get::<Alarm, _>(), Reading::Stale(&Alarm(false)));
}

#[test]
fn feeding_the_current_value_keeps_dependents_fresh() {
    let mut formation = Formation::new()
        .input(Temperature(20))
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10));
    poll(&mut formation);

    formation.feed(Temperature(20));

    assert!(formation.get::<AntiFreeze, _>().is_fresh());
}

#[test]
fn strata_deposited_after_a_poll_are_reported_by_the_next() {
    let mut formation = Formation::new().input(Temperature(20));
    poll(&mut formation);

    let mut formation = formation
        .input(Setpoint(18))
        .memo(|t: &Temperature| AntiFreeze(t.0 < 10));

    assert_eq!(poll(&mut formation), ["Setpoint(18)", "AntiFreeze(false)"]);
}
