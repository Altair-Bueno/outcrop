//! Changes travel the whole length of a chain of signals.

mod common;

use common::poll;
use outcrop::Formation;

#[derive(Debug, PartialEq, Eq)]
struct ModbusReg(u16);

#[derive(Debug, PartialEq, Eq)]
struct Temperature(i16);

#[derive(Debug, PartialEq, Eq)]
struct Celsius(i16);

#[derive(Debug, PartialEq, Eq)]
struct AntiFreezeAvail(bool);

fn temperature(r: &ModbusReg) -> Temperature {
    Temperature(r.0.cast_signed() / 10)
}

fn celsius(t: &Temperature) -> Celsius {
    Celsius(t.0)
}

fn anti_freeze(c: &Celsius) -> AntiFreezeAvail {
    AntiFreezeAvail(c.0 < 3)
}

#[test]
fn memo_chains_propagate_to_the_end() {
    let mut formation = Formation::new()
        .input(ModbusReg(200))
        .memo(temperature)
        .memo(celsius)
        .memo(anti_freeze);
    poll(&mut formation);

    formation.feed(ModbusReg(10));

    assert_eq!(
        poll(&mut formation),
        [
            "ModbusReg(10)",
            "Temperature(1)",
            "Celsius(1)",
            "AntiFreezeAvail(true)"
        ]
    );
}

#[test]
fn views_in_the_middle_of_a_chain_propagate_to_the_end() {
    let mut formation = Formation::new()
        .input(ModbusReg(200))
        .view(temperature)
        .memo(celsius)
        .memo(anti_freeze);
    poll(&mut formation);

    formation.feed(ModbusReg(10));

    assert_eq!(
        poll(&mut formation),
        [
            "ModbusReg(10)",
            "Temperature(1)",
            "Celsius(1)",
            "AntiFreezeAvail(true)"
        ]
    );
}

#[test]
fn views_over_memos_in_the_middle_of_a_chain_propagate_to_the_end() {
    let mut formation = Formation::new()
        .input(ModbusReg(200))
        .memo(temperature)
        .view(celsius)
        .memo(anti_freeze);
    poll(&mut formation);

    formation.feed(ModbusReg(10));

    assert_eq!(
        poll(&mut formation),
        [
            "ModbusReg(10)",
            "Temperature(1)",
            "Celsius(1)",
            "AntiFreezeAvail(true)"
        ]
    );
}
