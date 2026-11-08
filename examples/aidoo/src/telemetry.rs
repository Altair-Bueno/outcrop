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
pub(crate) struct Setpoint(pub(crate) DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutdoorTemperature(pub(crate) DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReturnTemperature(pub(crate) DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndoorExchangerTemperature(pub(crate) DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndoorGasPipeTemperature(pub(crate) DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutdoorExchangerTemperature(pub(crate) DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct DischargeTemperature(pub(crate) DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct OutdoorExpansionValve(pub(crate) Pulses);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct IndoorExpansionValve(pub(crate) Pulses);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EvaporationPressure(pub(crate) CentiMegapascal);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CondensationPressure(pub(crate) CentiMegapascal);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Current(pub(crate) DeciAmpere);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct WorkTemperature(pub(crate) DeciCelsius);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModbusAddress(pub(crate) u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeatingPower(pub(crate) Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CoolingPower(pub(crate) Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PhotovoltaicPower(pub(crate) Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeatPumpPower(pub(crate) Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct BuildingPower(pub(crate) Watts);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeatingGenerated(pub(crate) KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CoolingGenerated(pub(crate) KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Produced(pub(crate) KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HeatPumpConsumed(pub(crate) KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResistorHeatingConsumed(pub(crate) KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompressorHeatingConsumed(pub(crate) KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CompressorCoolingConsumed(pub(crate) KilowattHours);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Consumed(pub(crate) KilowattHours);

/// The temperature where the unit is, or `None` when it reads 0: the map
/// states that a reading is always greater than 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RoomTemperature(DeciCelsius);

impl RoomTemperature {
    pub(crate) fn new(register: &register::RoomTemperature) -> Option<Self> {
        (register.0 != 0).then(|| Self(DeciCelsius::from(register.0)))
    }
}

/// Decodes a register holding 0 (inactive) or 1 (active).
pub(crate) const fn flag(word: u16) -> Option<bool> {
    match word {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct EmergencyHeat(pub(crate) bool);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct T1T2Input(pub(crate) bool);

/// Error code of the unit, as four ASCII characters, or `None` when both
/// registers are zero, which means there is no error.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnitError([u8; 4]);

impl UnitError {
    pub(crate) fn new(
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
pub(crate) struct ErrorValue(NonZeroU16);

impl ErrorValue {
    pub(crate) fn new(register: &register::ErrorValue) -> Option<Self> {
        NonZeroU16::new(register.0).map(Self)
    }
}
