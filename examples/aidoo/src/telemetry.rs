//! Signals the unit reports: measurements, totals, flags and its error code.

use crate::register;
use crate::units::CentiMegapascal;
use crate::units::DeciAmpere;
use crate::units::DeciCelsius;
use crate::units::KilowattHours;
use crate::units::Pulses;
use crate::units::Watts;
use core::fmt;
use core::fmt::Debug;
use core::num::NonZeroU16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Setpoint(pub DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutdoorTemperature(pub DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReturnTemperature(pub DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndoorExchangerTemperature(pub DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndoorGasPipeTemperature(pub DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutdoorExchangerTemperature(pub DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DischargeTemperature(pub DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutdoorExpansionValve(pub Pulses);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IndoorExpansionValve(pub Pulses);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EvaporationPressure(pub CentiMegapascal);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CondensationPressure(pub CentiMegapascal);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Current(pub DeciAmpere);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkTemperature(pub DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModbusAddress(pub u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeatingPower(pub Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoolingPower(pub Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PhotovoltaicPower(pub Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeatPumpPower(pub Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuildingPower(pub Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeatingGenerated(pub KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoolingGenerated(pub KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Produced(pub KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeatPumpConsumed(pub KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResistorHeatingConsumed(pub KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressorHeatingConsumed(pub KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompressorCoolingConsumed(pub KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Consumed(pub KilowattHours);

/// The temperature where the unit is, or `None` when it reads 0: the map
/// states that a reading is always greater than 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoomTemperature(DeciCelsius);

impl RoomTemperature {
    pub fn new(register: &register::RoomTemperature) -> Option<Self> {
        (register.0 != 0).then(|| Self(DeciCelsius::from(register.0)))
    }
}

/// Decodes a register holding 0 (inactive) or 1 (active).
pub const fn flag(word: u16) -> Option<bool> {
    match word {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EmergencyHeat(pub bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct T1T2Input(pub bool);

/// Error code of the unit, as four ASCII characters, or `None` when both
/// registers are zero, which means there is no error.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UnitError([u8; 4]);

impl UnitError {
    pub fn new(
        first: &register::UnitErrorFirst,
        second: &register::UnitErrorSecond,
    ) -> Option<Self> {
        let [a, b] = first.0.to_be_bytes();
        let [c, d] = second.0.to_be_bytes();
        Some(Self([a, b, c, d])).filter(|error| error.0 != [0; 4])
    }
}

impl Debug for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnitError(\"{}\")", self.0.escape_ascii())
    }
}

/// Value of the unit's error, or `None` when it is zero, which means there is
/// no error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorValue(NonZeroU16);

impl ErrorValue {
    pub fn new(register: &register::ErrorValue) -> Option<Self> {
        NonZeroU16::new(register.0).map(Self)
    }
}
