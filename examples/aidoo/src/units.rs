//! Physical units, as scaled by the device.

use core::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeciCelsius(i16);

impl From<u16> for DeciCelsius {
    fn from(word: u16) -> Self {
        Self(word.cast_signed())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CentiMegapascal(u16);

impl From<u16> for CentiMegapascal {
    fn from(word: u16) -> Self {
        Self(word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DeciAmpere(u16);

impl From<u16> for DeciAmpere {
    fn from(word: u16) -> Self {
        Self(word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Watts(u16);

impl From<u16> for Watts {
    fn from(word: u16) -> Self {
        Self(word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pulses(u16);

impl From<u16> for Pulses {
    fn from(word: u16) -> Self {
        Self(word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KilowattHours(u32);

impl KilowattHours {
    pub fn new(low: u16, high: u16) -> Self {
        Self(u32::from(high) << 16 | u32::from(low))
    }
}
