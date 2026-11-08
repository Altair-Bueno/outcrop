//! Structs declared with `strata!`, nested in formations built by the
//! builder.
#![expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "derived signals receive their dependencies by reference"
)]

use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Reading;
use outcrop::Report;
use outcrop::Slot;
use outcrop::Visit;
use outcrop::Write;

#[derive(Debug, PartialEq)]
struct Label(&'static str);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Kelvin(u16);

#[derive(Debug, PartialEq)]
struct Celsius(i32);

#[derive(Debug, PartialEq)]
struct Freezing(bool);

#[derive(Debug, PartialEq)]
struct Offset(i32);

#[derive(Debug, PartialEq)]
struct Corrected(i32);

#[derive(Debug, PartialEq)]
struct Register<const ADDRESS: usize>(u16);

impl<const ADDRESS: usize> Slot for Register<ADDRESS> {
    type Item = u16;
    const INDEX: usize = ADDRESS;

    fn read(item: &u16) -> Self {
        Self(*item)
    }
}

fn celsius(k: &Kelvin) -> Celsius {
    Celsius(i32::from(k.0) - 273)
}

outcrop::strata! {
    struct Probe {
        kelvin: Kelvin = Kelvin(293),
        celsius: Celsius = Kelvin => celsius,
        Freezing = Celsius => |c| Freezing(c.0 <= 0),
        #[strata(bank)]
        registers: [u16; 3] = [0; 3],
    }
}

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

#[test]
fn first_poll_reports_every_nested_signal_in_place() {
    let mut formation = Formation::new()
        .input(Label("a"))
        .nest(Probe::default())
        .input(Offset(0));

    assert_eq!(
        poll(&mut formation),
        [
            "Label(\"a\")",
            "Kelvin(293)",
            "Celsius(20)",
            "Freezing(false)",
            "Element { index: 0, value: 0 }",
            "Element { index: 1, value: 0 }",
            "Element { index: 2, value: 0 }",
            "Offset(0)",
        ]
    );
}

#[test]
fn signals_above_depend_on_nested_signals() {
    let mut formation = Formation::new()
        .input(Offset(1))
        .nest(Probe::default())
        .memo(|c: &Celsius, o: &Offset| Corrected(c.0 + o.0));
    poll(&mut formation);

    formation.feed(Kelvin(263));

    assert_eq!(
        poll(&mut formation),
        [
            "Kelvin(263)",
            "Celsius(-10)",
            "Freezing(true)",
            "Corrected(-9)"
        ]
    );
}

#[test]
fn signals_above_are_recomputed_only_when_their_nested_dependency_changed() {
    let mut formation = Formation::new()
        .nest(Probe::default())
        .view(|r: &Register<1>| Corrected(i32::from(r.0)));
    poll(&mut formation);

    formation.feed(Write::new(0, &[5]));

    assert_eq!(poll(&mut formation), ["Element { index: 0, value: 5 }"]);

    formation.feed(Write::new(1, &[7]));

    assert_eq!(
        poll(&mut formation),
        ["Element { index: 1, value: 7 }", "Corrected(7)"]
    );
}

#[test]
fn nested_signals_are_read_by_type() {
    let mut formation = Formation::new().nest(Probe::default());
    poll(&mut formation);
    formation.feed(Kelvin(373));

    assert_eq!(formation.get::<Kelvin, _>(), Reading::Fresh(&Kelvin(373)));
    assert_eq!(formation.get::<Celsius, _>(), Reading::Stale(&Celsius(20)));
    assert_eq!(
        formation.get::<Freezing, _>(),
        Reading::Stale(Freezing(false))
    );

    poll(&mut formation);

    assert_eq!(formation.get::<Celsius, _>(), Reading::Fresh(&Celsius(100)));
}

#[test]
fn routers_above_feed_nested_inputs() {
    let mut formation = Formation::new()
        .nest(Probe::default())
        .route(|k: &u16, sink| sink.feed(Kelvin(*k)));
    poll(&mut formation);

    formation.feed(&273_u16);

    assert_eq!(
        poll(&mut formation),
        ["Kelvin(273)", "Celsius(0)", "Freezing(true)"]
    );
}

#[test]
fn signals_around_a_nested_struct_keep_their_own_bits() {
    let mut formation = Formation::new()
        .input(Label("a"))
        .nest(Probe::default())
        .input(Offset(0));
    poll(&mut formation);

    formation.feed(Offset(2));
    formation.feed(Label("b"));

    assert_eq!(poll(&mut formation), ["Label(\"b\")", "Offset(2)"]);
}
