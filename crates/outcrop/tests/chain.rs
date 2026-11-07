//! Changes travel the whole length of a chain of signals.

use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Report;
use outcrop::Visit;

#[derive(Debug, PartialEq)]
struct ModbusReg(u16);

#[derive(Debug, PartialEq)]
struct Temperature(i16);

#[derive(Debug, PartialEq)]
struct Celsius(i16);

#[derive(Debug, PartialEq)]
struct AntiFreezeAvail(bool);

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
