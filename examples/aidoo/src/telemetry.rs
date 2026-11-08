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

/// Signals decoded from a single register into a unit.
macro_rules! measurements {
    ($($name:ident($unit:ident) = $register:ty;)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) struct $name($unit);

        impl From<&$register> for $name {
            fn from(register: &$register) -> Self {
                Self($unit::from(register.0))
            }
        }
    )*};
}

measurements! {
    Setpoint(DeciCelsius) = register::Setpoint;
    OutdoorTemperature(DeciCelsius) = register::OutdoorTemperature;
    ReturnTemperature(DeciCelsius) = register::ReturnTemperature;
    IndoorExchangerTemperature(DeciCelsius) = register::IndoorExchangerTemperature;
    IndoorGasPipeTemperature(DeciCelsius) = register::IndoorGasPipeTemperature;
    OutdoorExchangerTemperature(DeciCelsius) = register::OutdoorExchangerTemperature;
    DischargeTemperature(DeciCelsius) = register::DischargeTemperature;
    OutdoorExpansionValve(Pulses) = register::OutdoorExpansionValve;
    IndoorExpansionValve(Pulses) = register::IndoorExpansionValve;
    EvaporationPressure(CentiMegapascal) = register::EvaporationPressure;
    CondensationPressure(CentiMegapascal) = register::CondensationPressure;
    Current(DeciAmpere) = register::Current;
    WorkTemperature(DeciCelsius) = register::WorkTemperature;
    ModbusAddress(u16) = register::ModbusAddress;
    HeatingPower(Watts) = register::HeatingPower;
    CoolingPower(Watts) = register::CoolingPower;
    PhotovoltaicPower(Watts) = register::PhotovoltaicPower;
    HeatPumpPower(Watts) = register::HeatPumpPower;
    BuildingPower(Watts) = register::BuildingPower;
}

/// Signals decoded from a low and a high register into a 32-bit total.
macro_rules! totals {
    ($($name:ident = $low:ty, $high:ty;)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) struct $name(KilowattHours);

        impl $name {
            pub(crate) fn new(low: &$low, high: &$high) -> Self {
                Self(KilowattHours::new(low.0, high.0))
            }
        }
    )*};
}

totals! {
    HeatingGenerated = register::HeatingGeneratedLow, register::HeatingGeneratedHigh;
    CoolingGenerated = register::CoolingGeneratedLow, register::CoolingGeneratedHigh;
    Produced = register::ProducedLow, register::ProducedHigh;
    HeatPumpConsumed = register::HeatPumpConsumedLow, register::HeatPumpConsumedHigh;
    ResistorHeatingConsumed = register::ResistorHeatingConsumedLow, register::ResistorHeatingConsumedHigh;
    CompressorHeatingConsumed = register::CompressorHeatingConsumedLow, register::CompressorHeatingConsumedHigh;
    CompressorCoolingConsumed = register::CompressorCoolingConsumedLow, register::CompressorCoolingConsumedHigh;
    Consumed = register::ConsumedLow, register::ConsumedHigh;
}

/// The temperature where the unit is, or `None` when it reads 0: the map
/// states that a reading is always greater than 0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct RoomTemperature(DeciCelsius);

impl RoomTemperature {
    pub(crate) fn new(register: &register::RoomTemperature) -> Option<Self> {
        (register.0 != 0).then(|| Self(DeciCelsius::from(register.0)))
    }
}

/// Signals decoded from a register holding 0 (inactive) or 1 (active).
macro_rules! flags {
    ($($name:ident = $register:ty;)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) struct $name(bool);

        impl $name {
            pub(crate) const fn new(register: &$register) -> Option<Self> {
                match register.0 {
                    0 => Some(Self(false)),
                    1 => Some(Self(true)),
                    _ => None,
                }
            }
        }
    )*};
}

flags! {
    EmergencyHeat = register::EmergencyHeat;
    T1T2Input = register::T1T2Input;
}

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

/// Value of the unit's error, or `None` when it is zero, which means there is
/// no error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ErrorValue(NonZeroU16);

impl ErrorValue {
    pub(crate) const fn new(register: &register::ErrorValue) -> Option<Self> {
        match NonZeroU16::new(register.0) {
            Some(value) => Some(Self(value)),
            None => None,
        }
    }
}

impl Debug for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnitError(\"{}\")", self.0.escape_ascii())
    }
}
