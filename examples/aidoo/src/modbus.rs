//! The unit's own Modbus settings.

use crate::register;
use crate::register::Invalid;
use core::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BaudRate(u32);

impl TryFrom<&register::BaudRate> for BaudRate {
    type Error = Invalid;

    fn try_from(register: &register::BaudRate) -> Result<Self, Invalid> {
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
            other => return Err(Invalid(other)),
        };
        Ok(Self(rate))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Parity {
    None,
    Odd,
    Even,
}

impl TryFrom<&register::Parity> for Parity {
    type Error = Invalid;

    fn try_from(register: &register::Parity) -> Result<Self, Invalid> {
        match register.0 {
            0 => Ok(Self::None),
            1 => Ok(Self::Odd),
            2 => Ok(Self::Even),
            other => Err(Invalid(other)),
        }
    }
}
