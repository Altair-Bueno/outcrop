//! The unit's own Modbus settings.

use crate::register;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaudRate(u32);

impl BaudRate {
    pub const fn new(register: &register::BaudRate) -> Option<Self> {
        let rate = match register.0 {
            0 => 100,
            1 => 300,
            2 => 500,
            3 => 1200,
            4 => 2400,
            5 => 4800,
            6 => 7800,
            7 => 9600,
            8 => 19200,
            9 => 57600,
            10 => 115_200,
            _ => return None,
        };
        Some(Self(rate))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Parity {
    None,
    Odd,
    Even,
}

impl Parity {
    pub const fn new(register: &register::Parity) -> Option<Self> {
        match register.0 {
            0 => Some(Self::None),
            1 => Some(Self::Odd),
            2 => Some(Self::Even),
            _ => None,
        }
    }
}
