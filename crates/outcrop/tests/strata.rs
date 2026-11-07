//! Formations declared with `strata!`.

use core::fmt::Debug;

use outcrop as layers;
use outcrop::Formation;
use outcrop::Reading;
use outcrop::Report;
use outcrop::Strata;
use outcrop::Visit;

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

#[derive(Debug, PartialEq)]
struct ModbusRegister {
    address: u16,
    value: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct Register<const ADDRESS: u16>(u16);

#[expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "derived signals receive their dependencies by reference"
)]
fn heating(t: &Temperature, s: &Setpoint) -> Heating {
    Heating(t.0 < s.0)
}

fn decode(chunk: [u8; 4]) -> ModbusRegister {
    ModbusRegister {
        address: u16::from_be_bytes([chunk[0], chunk[1]]),
        value: u16::from_be_bytes([chunk[2], chunk[3]]),
    }
}

outcrop::strata! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    struct Thermostat {
        Temperature = Temperature(20),
        Setpoint = Setpoint(18),
        Heating = (Temperature, Setpoint) => heating,
        AntiFreeze = Temperature => |t| AntiFreeze(t.0 < 10),
        Alarm = (Heating, AntiFreeze) => |h: &Heating, a| Alarm(h.0 && a.0),
    }
}

outcrop::strata! {
    struct Modbus {
        Register<0xAA> = Register(0),
        Register<0xAB> = Register(0),
        &ModbusRegister => |r, sink| match r.address {
            0xAA => sink.feed(Register::<0xAA>(r.value)),
            0xAB => sink.feed(Register::<0xAB>(r.value)),
            _ => {}
        },
        &[u8] => |frame, sink| {
            for chunk in frame.as_chunks::<4>().0 {
                sink.feed(&decode(*chunk));
            }
        },
    }
}

outcrop::strata! {
    struct Bits {
        Register<0> = Register(0),
        Register<1> = Register(0),
        Register<2> = Register(0),
        Register<3> = Register(0),
        Register<4> = Register(0),
        Register<5> = Register(0),
        Register<6> = Register(0),
        Register<7> = Register(0),
        Register<8> = Register(0),
        Setpoint = (Register<8>) => |high| Setpoint(i16::from(high.0 != 0)),
    }
}

outcrop::strata! {
    struct Shifted {
        #[cfg(any())]
        Missing = Missing,
        Register<0> = Register(0),
        Register<1> = Register(0),
        Register<2> = Register(0),
        Register<3> = Register(0),
        #[cfg(any())]
        Missing2 = Missing2,
        Register<4> = Register(0),
        Register<5> = Register(0),
        Register<6> = Register(0),
        Setpoint = (Register<6>) => |r| Setpoint(r.0.cast_signed()),
    }
}

outcrop::strata! {
    #[derive(Debug)]
    struct Named {
        temperature: Temperature = Temperature(20),
        Setpoint = Setpoint(18),
        #[cfg(all())]
        anti_freeze: AntiFreeze = Temperature => |t| AntiFreeze(t.0 < 10),
    }
}

#[derive(Debug, PartialEq)]
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
        Setpoint = Setpoint(18),
        Temperature = s0,
        V = Temperature => a0,
    }
}

outcrop::strata! {
    struct AbsentSignal {
        #[cfg(any())]
        Temperature = Temperature(0),
    }
}

outcrop::strata! {
    struct Gated {
        #[cfg(all())]
        Temperature = Temperature(20),
        #[cfg(any())]
        Missing = Missing,
        #[cfg(all())]
        AntiFreeze = Temperature => |t| AntiFreeze(t.0 < 10),
        #[cfg(any())]
        Alarm = Missing => |_| Alarm(true),
        #[cfg(any())]
        &[u8] => |_, _| {},
        #[cfg(all())]
        i16 => |value, sink| sink.feed(Temperature(value)),
    }
}

outcrop::strata! {
    #[cfg(any())]
    struct Absent {
        Temperature = Temperature(0),
    }
}

outcrop::strata! {
    #[strata(crate = layers)]
    #[derive(Debug)]
    struct Renamed {
        Temperature = Temperature(7),
    }
}

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

outcrop::strata! {
    struct UnitType {
        () = (),
        u32 = () => |_| 10,
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
fn named_fields_show_in_field_derives() {
    assert_eq!(
        format!("{:?}", Named::default()),
        "Named { temperature: Temperature(20), s1: Setpoint(18), anti_freeze: AntiFreeze(false) }"
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
fn cfg_removes_absent_signals() {
    let mut formation = Formation::from(AbsentSignal::default());

    assert_eq!(poll(&mut formation), Vec::<String>::new());
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

    formation.feed(3_i16);

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

    assert_eq!(formation.get(), Reading::Fresh(&Temperature(20)));
    assert_eq!(formation.get(), Reading::Stale(&Alarm(false)));

    poll(&mut formation);

    assert_eq!(formation.get(), Reading::Fresh(&Alarm(false)));
}

#[test]
fn feeding_marks_transitive_dependents_stale_until_the_next_poll() {
    let mut formation = Formation::from(Thermostat::default());
    poll(&mut formation);

    formation.feed(Setpoint(25));

    assert_eq!(formation.get(), Reading::Fresh(&Setpoint(25)));
    assert_eq!(formation.get(), Reading::Stale(&Heating(false)));
    assert_eq!(formation.get(), Reading::Fresh(&AntiFreeze(false)));
    assert_eq!(formation.get(), Reading::Stale(&Alarm(false)));

    poll(&mut formation);

    assert_eq!(formation.get(), Reading::Fresh(&Heating(true)));
    assert_eq!(formation.get(), Reading::Fresh(&Alarm(false)));
}

#[test]
fn signals_fed_through_routers_mark_dependents_stale() {
    let mut formation = Formation::from(Gated::default());
    poll(&mut formation);

    formation.feed(3_i16);

    assert_eq!(formation.get(), Reading::Fresh(&Temperature(3)));
    assert_eq!(formation.get(), Reading::Stale(&AntiFreeze(false)));
}
