//! Views: signals computed every time they are read, never stored.

use core::cell::Cell;
use core::fmt::Debug;
use core::mem::size_of_val;

use outcrop::Formation;
use outcrop::Reading;
use outcrop::Report;
use outcrop::Visit;

#[derive(Debug, PartialEq)]
struct Register<const ADDRESS: u16>(u16);

#[derive(Debug, PartialEq)]
struct AntiFreezeSupported(bool);

#[derive(Debug, PartialEq)]
struct Name([u8; 4]);

#[derive(Debug, PartialEq)]
struct Doubled(u32);

#[derive(Debug, PartialEq)]
struct Even(bool);

#[derive(Default)]
struct Recorder(Vec<String>);

impl<T: Debug> Visit<T> for Recorder {
    fn visit(&mut self, value: &T) {
        self.0.push(format!("{value:?}"));
    }
}

fn poll<S: Report<Recorder>>(formation: &mut Formation<S>) -> Vec<String> {
    let mut recorder = Recorder::default();
    formation.poll(&mut recorder);
    recorder.0
}

fn anti_freeze(r: &Register<0x42>) -> AntiFreezeSupported {
    AntiFreezeSupported(r.0 & 0x01 != 0)
}

fn name(high: &Register<0x10>, low: &Register<0x11>) -> Name {
    let [a, b] = high.0.to_be_bytes();
    let [c, d] = low.0.to_be_bytes();
    Name([a, b, c, d])
}

#[test]
fn first_poll_reports_views() {
    let mut formation = Formation::new()
        .input(Register::<0x42>(0x22))
        .view(anti_freeze);

    assert_eq!(
        poll(&mut formation),
        ["Register(34)", "AntiFreezeSupported(false)"]
    );
}

#[test]
fn views_are_reported_whenever_a_dependency_changes() {
    let mut formation = Formation::new()
        .input(Register::<0x42>(0x22))
        .view(anti_freeze);
    poll(&mut formation);

    formation.feed(Register::<0x42>(0x26));
    assert_eq!(
        poll(&mut formation),
        ["Register(38)", "AntiFreezeSupported(false)"]
    );

    formation.feed(Register::<0x42>(0x26));
    assert_eq!(poll(&mut formation), Vec::<String>::new());

    formation.feed(Register::<0x42>(0x27));
    assert_eq!(
        poll(&mut formation),
        ["Register(39)", "AntiFreezeSupported(true)"]
    );
}

#[test]
fn views_take_no_memory() {
    let inputs = Formation::new().input(Register::<0x42>(0x22));
    let viewed = Formation::new()
        .input(Register::<0x42>(0x22))
        .view(anti_freeze);

    assert_eq!(size_of_val(&viewed), size_of_val(&inputs));
}

#[test]
fn views_combine_several_signals() {
    let mut formation = Formation::new()
        .input(Register::<0x10>(u16::from_be_bytes(*b"ou")))
        .input(Register::<0x11>(u16::from_be_bytes(*b"tc")))
        .view(name);
    poll(&mut formation);

    formation.feed(Register::<0x11>(u16::from_be_bytes(*b"tr")));

    assert_eq!(formation.get::<Name, _>(), Reading::Fresh(Name(*b"outr")));
    assert_eq!(
        poll(&mut formation),
        ["Register(29810)", "Name([111, 117, 116, 114])"]
    );
}

#[test]
fn views_follow_memos_recomputed_by_the_poll() {
    let mut formation = Formation::new()
        .input(Register::<0x42>(1))
        .memo(|r: &Register<0x42>| Doubled(u32::from(r.0) * 2))
        .view(|d: &Doubled| Even(d.0.is_multiple_of(4)));
    poll(&mut formation);

    formation.feed(Register::<0x42>(2));

    assert_eq!(formation.get::<Even, _>(), Reading::Stale(Even(false)));
    assert_eq!(
        poll(&mut formation),
        ["Register(2)", "Doubled(4)", "Even(true)"]
    );
    assert_eq!(formation.get::<Even, _>(), Reading::Fresh(Even(true)));
}

#[test]
fn memos_above_views_recompute_but_stop_unchanged_values() {
    let calls = Cell::new(0);
    let mut formation = Formation::new()
        .input(Register::<0x42>(0x22))
        .view(anti_freeze)
        .memo(|a: &AntiFreezeSupported| {
            calls.set(calls.get() + 1);
            Even(a.0)
        });
    poll(&mut formation);
    let before = calls.get();

    formation.feed(Register::<0x42>(0x26));
    assert_eq!(formation.get::<Even, _>(), Reading::Stale(&Even(false)));
    assert_eq!(
        poll(&mut formation),
        ["Register(38)", "AntiFreezeSupported(false)"]
    );
    assert_eq!(calls.get(), before + 1);

    formation.feed(Register::<0x42>(0x27));
    assert_eq!(
        poll(&mut formation),
        ["Register(39)", "AntiFreezeSupported(true)", "Even(true)"]
    );
    assert_eq!(calls.get(), before + 2);
}

#[test]
fn views_build_on_views() {
    let mut formation = Formation::new()
        .input(Register::<0x42>(0x22))
        .view(anti_freeze)
        .view(|a: &AntiFreezeSupported, r: &Register<0x42>| Even(a.0 && r.0 > 0x30));
    poll(&mut formation);

    formation.feed(Register::<0x42>(0x23));
    assert_eq!(
        poll(&mut formation),
        ["Register(35)", "AntiFreezeSupported(true)", "Even(false)"]
    );

    formation.feed(Register::<0x42>(0x31));
    assert_eq!(
        poll(&mut formation),
        ["Register(49)", "AntiFreezeSupported(true)", "Even(true)"]
    );
}

#[test]
fn routers_feed_through_views() {
    let mut formation = Formation::new()
        .input(Register::<0x42>(0x22))
        .view(anti_freeze)
        .route(|value: &u16, out| out.feed(Register::<0x42>(*value)));
    poll(&mut formation);

    formation.feed(&0x23);

    assert_eq!(
        poll(&mut formation),
        ["Register(35)", "AntiFreezeSupported(true)"]
    );
}

#[test]
fn views_above_routers_see_routed_values() {
    let mut formation = Formation::new()
        .input(Register::<0x42>(0x22))
        .route(|value: &u16, out| out.feed(Register::<0x42>(*value)))
        .view(anti_freeze);
    poll(&mut formation);

    formation.feed(&0x23);

    assert_eq!(
        poll(&mut formation),
        ["Register(35)", "AntiFreezeSupported(true)"]
    );
}

#[test]
fn views_need_no_partial_eq() {
    #[derive(Debug)]
    struct Label(&'static str);

    let mut formation = Formation::new()
        .input(Register::<0x42>(0x22))
        .view(|r: &Register<0x42>| Label(if r.0 & 0x01 != 0 { "on" } else { "off" }));
    poll(&mut formation);

    formation.feed(Register::<0x42>(0x23));

    assert_eq!(formation.get::<Label, _>().into_inner().0, "on");
    assert_eq!(poll(&mut formation), ["Register(35)", "Label(\"on\")"]);
}
