//! Behaviour of routers fanning one fed value out into typed inputs.

use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Visit;

#[derive(Debug, PartialEq)]
struct ModbusRegister {
    address: u16,
    value: u16,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Register<const ADDRESS: u16>(u16);

#[derive(Debug, Clone, Copy, PartialEq)]
struct Version {
    major: u16,
    minor: u16,
}

#[derive(Default)]
struct Recorder(Vec<String>);

impl<T: Debug> Visit<T> for Recorder {
    fn visit(&mut self, value: &T) {
        self.0.push(format!("{value:?}"));
    }
}

fn decode(chunk: &[u8]) -> ModbusRegister {
    ModbusRegister {
        address: u16::from_be_bytes([chunk[0], chunk[1]]),
        value: u16::from_be_bytes([chunk[2], chunk[3]]),
    }
}

macro_rules! modbus_formation {
    () => {
        Formation::new()
            .input(Register::<0xAA>(0))
            .input(Register::<0xAB>(0))
            .route(|r: &ModbusRegister, out| match r.address {
                0xAA => out.feed(Register::<0xAA>(r.value)),
                0xAB => out.feed(Register::<0xAB>(r.value)),
                _ => {}
            })
            .route(|frame: &[u8], out| {
                for chunk in frame.chunks_exact(4) {
                    out.feed(&decode(chunk));
                }
            })
            .memo(|aa: &Register<0xAA>, ab: &Register<0xAB>| Version {
                major: aa.0,
                minor: ab.0,
            })
    };
}

#[test]
fn routers_report_nothing_themselves() {
    let mut formation = modbus_formation!();
    let mut recorder = Recorder::default();

    formation.poll(&mut recorder);

    assert_eq!(
        recorder.0,
        [
            "Register(0)",
            "Register(0)",
            "Version { major: 0, minor: 0 }"
        ]
    );
}

#[test]
fn router_feeds_the_input_selected_at_runtime() {
    let mut formation = modbus_formation!();
    formation.poll(&mut Recorder::default());
    let mut recorder = Recorder::default();

    formation.feed(&ModbusRegister {
        address: 0xAB,
        value: 4,
    });
    formation.poll(&mut recorder);

    assert_eq!(
        recorder.0,
        ["Register(4)", "Version { major: 0, minor: 4 }"]
    );
}

#[test]
fn router_ignores_values_it_does_not_route() {
    let mut formation = modbus_formation!();
    formation.poll(&mut Recorder::default());
    let mut recorder = Recorder::default();

    formation.feed(&ModbusRegister {
        address: 0x01,
        value: 4,
    });
    formation.poll(&mut recorder);

    assert_eq!(recorder.0, Vec::<String>::new());
}

#[test]
fn routers_chain_from_borrowed_bytes_to_typed_signals() {
    let mut formation = modbus_formation!();
    formation.poll(&mut Recorder::default());
    let mut recorder = Recorder::default();

    let frame = [0x00, 0xAA, 0x00, 0x01, 0x00, 0xAB, 0x00, 0x04];
    formation.feed(&frame[..]);
    formation.poll(&mut recorder);

    assert_eq!(
        recorder.0,
        [
            "Register(1)",
            "Register(4)",
            "Version { major: 1, minor: 4 }"
        ]
    );
}

#[test]
fn clones_keep_what_their_routers_captured() {
    let allowed = Vec::from([0xAA]);
    let mut formation =
        Formation::new()
            .input(Register::<0xAA>(0))
            .route(move |r: &ModbusRegister, out| {
                if allowed.contains(&r.address) {
                    out.feed(Register::<0xAA>(r.value));
                }
            });
    formation.poll(&mut Recorder::default());
    let mut clone = formation.clone();
    let mut recorder = Recorder::default();

    clone.feed(&ModbusRegister {
        address: 0xAA,
        value: 7,
    });
    clone.poll(&mut recorder);
    formation.poll(&mut recorder);

    assert_eq!(recorder.0, ["Register(7)"]);
}
