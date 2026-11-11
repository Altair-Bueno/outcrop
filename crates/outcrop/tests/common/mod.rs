#![allow(
    dead_code,
    reason = "each test crate uses only some of the shared helpers"
)]

use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Report;
use outcrop::Slot;
use outcrop::Visit;

#[derive(Default)]
pub struct Recorder(pub Vec<String>);

impl<T: Debug> Visit<T> for Recorder {
    fn visit(&mut self, value: &T) {
        self.0.push(format!("{value:?}"));
    }
}

pub fn poll<S: Report<Recorder>>(formation: &mut Formation<S>) -> Vec<String> {
    let mut recorder = Recorder::default();
    formation.poll(&mut recorder);
    recorder.0
}

/// Panics on the first signal whose `Debug` output starts with its prefix.
pub struct PanicOn(pub &'static str);

impl<T: Debug> Visit<T> for PanicOn {
    fn visit(&mut self, value: &T) {
        assert!(!format!("{value:?}").starts_with(self.0), "visitor panics");
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Register<const ADDRESS: usize>(pub u16);

impl<const ADDRESS: usize> Slot for Register<ADDRESS> {
    type Item = u16;
    const INDEX: usize = ADDRESS;

    fn read(item: &u16) -> Self {
        Self(*item)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct ModbusRegister {
    pub address: u16,
    pub value: u16,
}

pub fn decode(chunk: &[u8]) -> ModbusRegister {
    ModbusRegister {
        address: u16::from_be_bytes([chunk[0], chunk[1]]),
        value: u16::from_be_bytes([chunk[2], chunk[3]]),
    }
}
