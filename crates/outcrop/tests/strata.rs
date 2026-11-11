//! Formations declared with `strata!`.

mod common;

use common::ModbusRegister;
use common::PanicOn;
use common::decode;
use common::poll;
use outcrop as layers;
use outcrop::Formation;
use outcrop::Reading;
use outcrop::Strata;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Temperature(i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Setpoint(i16);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Heating(bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AntiFreeze(bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Alarm(bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Register<const ADDRESS: u16>(u16);

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "derived signals receive their dependencies by reference"
)]
fn heating(t: &Temperature, s: &Setpoint) -> Heating {
    Heating(t.0 < s.0)
}

outcrop::strata! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct Thermostat {
        temperature: Temperature = Temperature(20),
        setpoint: Setpoint = Setpoint(18),
        heating: Heating = (Temperature, Setpoint) => heating,
        anti_freeze: AntiFreeze = Temperature => |t| AntiFreeze(t.0 < 10),
        alarm: Alarm = (Heating, AntiFreeze) => |h: &Heating, a| Alarm(h.0 && a.0),
    }
}

outcrop::strata! {
    struct Modbus {
        aa: Register<0xAA> = Register(0),
        ab: Register<0xAB> = Register(0),
        ModbusRegister => |r, sink| match r.address {
            0xAA => sink.feed(Register::<0xAA>(r.value)),
            0xAB => sink.feed(Register::<0xAB>(r.value)),
            _ => {}
        },
        [u8] => |frame, sink| {
            for chunk in frame.as_chunks::<4>().0 {
                sink.feed(&decode(chunk));
            }
        },
    }
}

outcrop::strata! {
    struct Bits {
        r0: Register<0> = Register(0),
        r1: Register<1> = Register(0),
        r2: Register<2> = Register(0),
        r3: Register<3> = Register(0),
        r4: Register<4> = Register(0),
        r5: Register<5> = Register(0),
        r6: Register<6> = Register(0),
        r7: Register<7> = Register(0),
        r8: Register<8> = Register(0),
        setpoint: Setpoint = (Register<8>) => |high| Setpoint(i16::from(high.0 != 0)),
    }
}

outcrop::strata! {
    struct Shifted {
        #[cfg(any())]
        missing: Missing = Missing,
        r0: Register<0> = Register(0),
        r1: Register<1> = Register(0),
        r2: Register<2> = Register(0),
        r3: Register<3> = Register(0),
        #[cfg(any())]
        missing2: Missing2 = Missing2,
        r4: Register<4> = Register(0),
        r5: Register<5> = Register(0),
        r6: Register<6> = Register(0),
        setpoint: Setpoint = (Register<6>) => |r| Setpoint(r.0.cast_signed()),
    }
}

outcrop::strata! {
    #[derive(Debug)]
    struct Named {
        temperature: Temperature = Temperature(20),
        setpoint: Setpoint = Setpoint(18),
        #[cfg(all())]
        anti_freeze: AntiFreeze = Temperature => |t| AntiFreeze(t.0 < 10),
        Heating = (Temperature, Setpoint) => heating,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct AntiFreezeSupported(bool);

outcrop::strata! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct ModbusMap {
        r42: Register<0x42> = Register(0x22),
        r43: Register<0x43> = Register(0),
        AntiFreezeSupported = Register<0x42> => |r| AntiFreezeSupported(r.0 & 0x01 != 0),
        Setpoint = (Register<0x42>, Register<0x43>) => |a, b| {
            Setpoint(i16::from_be_bytes([a.0.to_be_bytes()[1], b.0.to_be_bytes()[1]]))
        },
        alarm: Alarm = (AntiFreezeSupported, Setpoint) => |a, s| Alarm(a.0 && s.0 > 0x2300),
        Heating = Alarm => |a| Heating(!a.0),
        ModbusRegister => |r, sink| match r.address {
            0x42 => sink.feed(Register::<0x42>(r.value)),
            0x43 => sink.feed(Register::<0x43>(r.value)),
            _ => {}
        },
    }
}

#[derive(Debug, PartialEq, Eq)]
struct V(i16);

#[expect(
    non_upper_case_globals,
    reason = "named like the temporaries `strata!` declares, which must not capture it"
)]
const s0: Temperature = Temperature(5);

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "derived signals receive their dependencies by reference"
)]
fn a0(t: &Temperature) -> V {
    V(t.0)
}

outcrop::strata! {
    struct Hygiene {
        setpoint: Setpoint = Setpoint(18),
        s0: Temperature = s0,
        a0: V = Temperature => a0,
    }
}

macro_rules! wrapped {
    ($input:ident, $memo:ident) => {
        outcrop::strata! {
            struct Wrapped {
                temperature: $input = $input(20),
                anti_freeze: $memo = $input => |t| $memo(t.0 < 10),
            }
        }
    };
}

wrapped!(Temperature, AntiFreeze);

type Celsius = Temperature;

type Threshold = Setpoint;

#[expect(
    unused_qualifications,
    reason = "signals are spelled differently on purpose"
)]
mod spelled {
    use super::AntiFreeze;
    use super::Celsius;
    use super::Heating;
    use super::Setpoint;
    use super::Temperature;
    use super::heating;

    outcrop::strata! {
        pub(crate) struct Spelled {
            temperature: Celsius = Temperature(20),
            setpoint: super::Threshold = Setpoint(18),
            heating: Heating = (Temperature, crate::Setpoint) => heating,
            AntiFreeze = self::Temperature => |t| AntiFreeze(t.0 < 10),
        }
    }
}

outcrop::strata! {
    struct AbsentSignal {
        #[cfg(any())]
        temperature: Temperature = Temperature(0),
    }
}

outcrop::strata! {
    struct Gated {
        #[cfg(all())]
        temperature: Temperature = Temperature(20),
        #[cfg(any())]
        missing: Missing = Missing,
        #[cfg(all())]
        anti_freeze: AntiFreeze = Temperature => |t| AntiFreeze(t.0 < 10),
        #[cfg(any())]
        Alarm = Missing => |_| Alarm(true),
        #[cfg(any())]
        [u8] => |_, _| {},
        #[cfg(all())]
        i16 => |value, sink| sink.feed(Temperature(*value)),
    }
}

outcrop::strata! {
    struct Conditional {
        #[cfg_attr(all(), cfg(all()))]
        temperature: Temperature = Temperature(20),
        #[cfg_attr(all(), cfg(any()))]
        missing: Missing = Missing,
        #[cfg_attr(all(), cfg(any()))]
        Alarm = Missing => |_| Alarm(true),
        #[cfg_attr(all(), cfg(any()))]
        Missing => |_, _| {},
        #[cfg(all())]
        #[strata(fn(expect(unused_variables, reason = "reaches the view's closure")))]
        AntiFreeze = Temperature => |t| AntiFreeze(false),
        #[deny(unsafe_code)]
        Heating = Temperature => |t| Heating(t.0 > 0),
    }
}

outcrop::strata! {
    struct Empty {}
}

outcrop::strata! {
    struct RouterOnly {
        u8 => |_, _| {},
    }
}

outcrop::strata! {
    struct Annotated {
        temperature: Temperature = Temperature(20),
        // The closures below ignore their arguments, so each `#[expect]` is
        // fulfilled only if it reaches the function generated from it.
        #[strata(fn(expect(unused_variables, reason = "reaches the memo's closure")))]
        #[strata(field(doc = "Docs go on the field when targeted at it."))]
        anti_freeze: AntiFreeze = Temperature => |t| AntiFreeze(false),
        #[strata(fn(expect(unused_variables, reason = "reaches the view's closure")))]
        Alarm = Temperature => |t| Alarm(true),
        #[strata(fn(expect(unused_variables, reason = "reaches the router's closure")))]
        i16 => |value, sink| sink.feed(Temperature(0)),
    }
}

#[cfg(any())]
outcrop::strata! {
    struct Absent {
        temperature: Temperature = Temperature(0),
    }
}

#[cfg_attr(all(), cfg(any()))]
outcrop::strata! {
    struct AbsentByAttr {
        temperature: Temperature = Missing,
    }
}

outcrop::strata! {
    #[strata(crate = layers)]
    #[derive(Debug)]
    struct Renamed {
        temperature: Temperature = Temperature(7),
    }
}

outcrop::strata! {
    struct UnitType {
        unit: () = (),
        u32 = () => |()| 10,
    }
}

#[test]
fn first_poll_reports_every_initial_value() {
    let mut formation = Formation::from(Thermostat::default());

    assert_eq!(
        poll(&mut formation),
        [
            "Temperature(20)",
            "Setpoint(18)",
            "Heating(false)",
            "AntiFreeze(false)",
            "Alarm(false)"
        ]
    );
    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn feeding_recomputes_dependents_in_order() {
    let mut formation = Formation::from(Thermostat::default());
    poll(&mut formation);

    formation.feed(Temperature(0));

    assert_eq!(
        poll(&mut formation),
        [
            "Temperature(0)",
            "Heating(true)",
            "AntiFreeze(true)",
            "Alarm(true)"
        ]
    );
}

#[test]
fn feeding_the_current_value_reports_nothing() {
    let mut formation = Formation::from(Thermostat::default());
    poll(&mut formation);

    formation.feed(Setpoint(18));

    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn equal_recomputations_stop_propagation() {
    let mut formation = Formation::from(Thermostat::default());
    poll(&mut formation);

    formation.feed(Setpoint(19));

    assert_eq!(poll(&mut formation), ["Setpoint(19)"]);
}

#[test]
fn only_stored_signals_show_in_field_derives() {
    assert_eq!(
        format!("{:?}", Named::default()),
        "Named { temperature: Temperature(20), setpoint: Setpoint(18), anti_freeze: AntiFreeze(false) }"
    );
}

#[test]
fn derives_apply_to_the_declared_struct() {
    let mut formation = Formation::from(Thermostat::default());
    let copy = formation;

    assert_eq!(formation, copy);
    poll(&mut formation);
    assert_ne!(formation, copy);
}

#[test]
fn routers_chain_from_bytes_to_registers() {
    let mut formation = Formation::from(Modbus::default());
    poll(&mut formation);

    formation.feed(
        &[
            0x00, 0xAA, 0x00, 0x07, 0x00, 0xAB, 0x00, 0x09, 0x00, 0xFF, 0x00, 0x01,
        ][..],
    );

    assert_eq!(poll(&mut formation), ["Register(7)", "Register(9)"]);
}

#[test]
fn signals_past_the_first_byte_are_tracked() {
    let mut formation = Formation::from(Bits::default());
    assert_eq!(poll(&mut formation).len(), 10);

    formation.feed(Register::<8>(1));

    assert_eq!(poll(&mut formation), ["Register(1)", "Setpoint(1)"]);
}

#[test]
fn generated_names_do_not_capture_user_code() {
    let mut formation = Formation::from(Hygiene::default());

    assert_eq!(
        poll(&mut formation),
        ["Setpoint(18)", "Temperature(5)", "V(5)"]
    );
}

#[test]
fn signals_are_found_by_type_however_they_are_spelled() {
    let mut formation = Formation::from(spelled::Spelled::default());
    poll(&mut formation);

    formation.feed(Temperature(5));
    formation.feed::<Threshold, _>(Setpoint(30));

    assert_eq!(
        poll(&mut formation),
        [
            "Temperature(5)",
            "Setpoint(30)",
            "Heating(true)",
            "AntiFreeze(true)"
        ]
    );
    assert_eq!(
        formation.get::<Celsius, _>(),
        Reading::Fresh(&Temperature(5))
    );
}

#[test]
fn signals_declared_through_other_macros_compare_their_values() {
    let mut formation = Formation::from(Wrapped::default());
    poll(&mut formation);

    formation.feed(Temperature(5));

    assert_eq!(poll(&mut formation), ["Temperature(5)", "AntiFreeze(true)"]);
}

#[test]
fn cfg_removes_absent_signals() {
    let mut formation = Formation::from(AbsentSignal::default());

    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn formations_without_signals_take_no_bits() {
    assert_eq!(size_of::<<Empty as Strata>::Flags>(), 0);
    assert_eq!(size_of::<<RouterOnly as Strata>::Flags>(), 0);

    let mut empty = Formation::from(Empty::default());
    let mut router = Formation::from(RouterOnly::default());
    router.feed(&1_u8);

    assert_eq!(poll(&mut empty), Vec::<String>::new());
    assert_eq!(poll(&mut router), Vec::<String>::new());
}

#[test]
fn attributes_reach_the_functions_they_target() {
    let mut formation = Formation::from(Annotated::default());
    formation.feed(&3_i16);

    assert_eq!(
        poll(&mut formation),
        ["Temperature(0)", "AntiFreeze(false)", "Alarm(true)"]
    );
}

#[test]
fn bare_attributes_apply_to_the_whole_entry() {
    assert_eq!(size_of::<<Conditional as Strata>::Flags>(), 1);

    let mut formation = Formation::from(Conditional::default());

    assert_eq!(
        poll(&mut formation),
        ["Temperature(20)", "AntiFreeze(false)", "Heating(true)"]
    );
}

#[test]
fn cfg_removed_signals_take_no_bits() {
    assert_eq!(size_of::<<AbsentSignal as Strata>::Flags>(), 0);
    assert_eq!(size_of::<<Shifted as Strata>::Flags>(), 1);

    let mut formation = Formation::from(Shifted::default());
    assert_eq!(poll(&mut formation).len(), 8);

    formation.feed(Register::<6>(3));

    assert_eq!(poll(&mut formation), ["Register(3)", "Setpoint(3)"]);
}

#[test]
fn cfg_keeps_present_signals_and_routers() {
    let mut formation = Formation::from(Gated::default());
    assert_eq!(
        poll(&mut formation),
        ["Temperature(20)", "AntiFreeze(false)"]
    );

    formation.feed(&3_i16);

    assert_eq!(poll(&mut formation), ["Temperature(3)", "AntiFreeze(true)"]);
}

#[test]
fn unit_is_a_signal_and_a_dependency() {
    let mut formation = Formation::from(UnitType::default());
    assert_eq!(poll(&mut formation), ["()", "10"]);

    formation.feed(());

    assert_eq!(poll(&mut formation), Vec::<String>::new());
}

#[test]
fn generated_code_uses_the_given_crate_path() {
    let mut formation = Formation::from(Renamed::default());

    assert_eq!(poll(&mut formation), ["Temperature(7)"]);
}

#[test]
fn derived_reads_are_stale_until_the_first_poll() {
    let mut formation = Formation::from(Thermostat::default());

    assert_eq!(
        formation.get::<Temperature, _>(),
        Reading::Fresh(&Temperature(20))
    );
    assert_eq!(formation.get::<Alarm, _>(), Reading::Stale(&Alarm(false)));

    poll(&mut formation);

    assert_eq!(formation.get::<Alarm, _>(), Reading::Fresh(&Alarm(false)));
}

#[test]
fn feeding_marks_transitive_dependents_stale_until_the_next_poll() {
    let mut formation = Formation::from(Thermostat::default());
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
    assert_eq!(formation.get::<Alarm, _>(), Reading::Stale(&Alarm(false)));

    poll(&mut formation);

    assert_eq!(
        formation.get::<Heating, _>(),
        Reading::Fresh(&Heating(true))
    );
    assert_eq!(formation.get::<Alarm, _>(), Reading::Fresh(&Alarm(false)));
}

#[test]
fn signals_fed_through_routers_mark_dependents_stale() {
    let mut formation = Formation::from(Gated::default());
    poll(&mut formation);

    formation.feed(&3_i16);

    assert_eq!(
        formation.get::<Temperature, _>(),
        Reading::Fresh(&Temperature(3))
    );
    assert_eq!(
        formation.get::<AntiFreeze, _>(),
        Reading::Stale(&AntiFreeze(false))
    );
}

#[test]
fn views_take_no_field() {
    assert_eq!(
        size_of::<ModbusMap>(),
        size_of::<(Register<0x42>, Register<0x43>, Alarm)>()
    );
}

#[test]
fn views_are_reported_whenever_a_dependency_changes() {
    let mut formation = Formation::from(ModbusMap::default());
    assert_eq!(
        poll(&mut formation),
        [
            "Register(34)",
            "Register(0)",
            "AntiFreezeSupported(false)",
            "Setpoint(8704)",
            "Alarm(false)",
            "Heating(true)"
        ]
    );

    formation.feed(Register::<0x42>(0x1122));
    assert_eq!(
        poll(&mut formation),
        [
            "Register(4386)",
            "AntiFreezeSupported(false)",
            "Setpoint(8704)"
        ]
    );

    formation.feed(Register::<0x42>(0x23));
    assert_eq!(
        poll(&mut formation),
        [
            "Register(35)",
            "AntiFreezeSupported(true)",
            "Setpoint(8960)"
        ]
    );
}

#[test]
fn memos_and_views_build_on_each_other() {
    let mut formation = Formation::from(ModbusMap::default());
    poll(&mut formation);

    formation.feed(&ModbusRegister {
        address: 0x42,
        value: 0x23,
    });
    formation.feed(&ModbusRegister {
        address: 0x43,
        value: 0x01,
    });

    assert_eq!(
        formation.get::<Setpoint, _>(),
        Reading::Fresh(Setpoint(0x2301))
    );
    assert_eq!(formation.get::<Alarm, _>(), Reading::Stale(&Alarm(false)));
    assert_eq!(formation.get::<Heating, _>(), Reading::Stale(Heating(true)));
    assert_eq!(
        poll(&mut formation),
        [
            "Register(35)",
            "Register(1)",
            "AntiFreezeSupported(true)",
            "Setpoint(8961)",
            "Alarm(true)",
            "Heating(false)"
        ]
    );
    assert_eq!(
        formation.get::<Heating, _>(),
        Reading::Fresh(Heating(false))
    );
}

#[test]
fn memos_above_views_are_stale_until_polled() {
    let mut formation = Formation::from(ModbusMap::default());
    poll(&mut formation);

    formation.feed(Register::<0x42>(0x1122));

    assert_eq!(formation.get::<Alarm, _>(), Reading::Stale(&Alarm(false)));
    poll(&mut formation);
    assert_eq!(formation.get::<Alarm, _>(), Reading::Fresh(&Alarm(false)));
}

#[derive(Debug)]
struct Label(&'static str);

outcrop::strata! {
    struct Labelled {
        r42: Register<0x42> = Register(0x22),
        Label = Register<0x42> => |r| Label(if r.0 & 0x01 != 0 { "on" } else { "off" }),
    }
}

#[test]
fn views_need_no_partial_eq() {
    let mut formation = Formation::from(Labelled::default());
    poll(&mut formation);

    formation.feed(Register::<0x42>(0x23));

    assert_eq!(formation.get::<Label, _>().into_inner().0, "on");
    assert_eq!(poll(&mut formation), ["Register(35)", "Label(\"on\")"]);
}

#[test]
fn views_see_the_last_of_several_feeds() {
    let mut formation = Formation::from(ModbusMap::default());
    poll(&mut formation);

    formation.feed(&ModbusRegister {
        address: 0x42,
        value: 0x24,
    });
    formation.feed(&ModbusRegister {
        address: 0x42,
        value: 0x25,
    });

    assert_eq!(
        poll(&mut formation),
        [
            "Register(37)",
            "AntiFreezeSupported(true)",
            "Setpoint(9472)",
            "Alarm(true)",
            "Heating(false)"
        ]
    );
}

outcrop::strata! {
    struct Internal {
        chain: Temperature = Temperature(20),
        f: Setpoint = Setpoint(18),
        a0: Heating = (Temperature, Setpoint) => heating,
    }
}

#[test]
fn fields_may_share_names_with_the_generated_code() {
    let mut formation = Formation::from(Internal::default());

    assert_eq!(
        poll(&mut formation),
        ["Temperature(20)", "Setpoint(18)", "Heating(false)"]
    );
}

outcrop::strata! {
    struct Interrupted {
        temperature: Temperature = Temperature(20),
        setpoint: Setpoint = Setpoint(18),
        heating: Heating = (Temperature, Setpoint) => heating,
    }
}

#[test]
fn a_panicking_visitor_leaves_the_signals_after_it_to_the_next_poll() {
    let mut formation = Formation::from(Interrupted::default());
    poll(&mut formation);
    formation.feed(Temperature(10));
    formation.feed(Setpoint(19));

    let panicked = std::panic::catch_unwind(core::panic::AssertUnwindSafe(|| {
        formation.poll(&mut PanicOn("Setpoint"));
    }));

    assert!(panicked.is_err());
    assert_eq!(poll(&mut formation), ["Heating(true)"]);
    assert_eq!(poll(&mut formation), [""; 0]);
}
