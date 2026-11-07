//! Formations declared with `strata!`.

use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Report;
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
