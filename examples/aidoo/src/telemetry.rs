//! Signals the unit reports: measurements, totals, flags and its error code.

use crate::register;
use crate::register::Invalid;
use crate::units::CentiMegapascal;
use crate::units::DeciAmpere;
use crate::units::DeciCelsius;
use crate::units::KilowattHours;
use crate::units::Pulses;
use crate::units::Watts;
use core::fmt;
use core::fmt::Debug;

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
    RoomTemperature(DeciCelsius) = register::RoomTemperature;
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
    ErrorValue(u16) = register::ErrorValue;
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

/// Signals decoded from a register holding 0 (inactive) or 1 (active).
macro_rules! flags {
    ($($name:ident = $register:ty;)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) struct $name(bool);

        impl TryFrom<&$register> for $name {
            type Error = Invalid;

            fn try_from(register: &$register) -> Result<Self, Invalid> {
                match register.0 {
                    0 => Ok(Self(false)),
                    1 => Ok(Self(true)),
                    other => Err(Invalid(other)),
                }
            }
        }
    )*};
}

flags! {
    EmergencyHeat = register::EmergencyHeat;
    T1T2Input = register::T1T2Input;
}

/// Error code of the unit, as four ASCII characters.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct UnitError([u8; 4]);

impl UnitError {
    pub(crate) fn new(
        first: &register::UnitErrorFirst,
        second: &register::UnitErrorSecond,
    ) -> Self {
        let [a, b] = first.0.to_be_bytes();
        let [c, d] = second.0.to_be_bytes();
        Self([a, b, c, d])
    }
}

impl Debug for UnitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "UnitError(\"{}\")", self.0.escape_ascii())
    }
}
