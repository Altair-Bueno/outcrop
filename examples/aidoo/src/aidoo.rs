//! The formation of an Aidoo: one input per register, and the signals decoded
//! from them.

use crate::capabilities::AutoLimits;
use crate::capabilities::AvailableLouvers;
use crate::capabilities::AvailableModes;
use crate::capabilities::AvailableSpeeds;
use crate::capabilities::CoolLimits;
use crate::capabilities::DryLimits;
use crate::capabilities::FanLimits;
use crate::capabilities::HeatLimits;
use crate::capabilities::Limits;
use crate::capabilities::SetpointRange;
use crate::capabilities::setpoint_range;
use crate::control::FanSpeed;
use crate::control::HorizontalLouvers;
use crate::control::Mode;
use crate::control::NumericFanSpeed;
use crate::control::Power;
use crate::control::VerticalLouvers;
use crate::modbus::BaudRate;
use crate::modbus::Parity;
use crate::register;
use crate::telemetry::BuildingPower;
use crate::telemetry::CompressorCoolingConsumed;
use crate::telemetry::CompressorHeatingConsumed;
use crate::telemetry::CondensationPressure;
use crate::telemetry::Consumed;
use crate::telemetry::CoolingGenerated;
use crate::telemetry::CoolingPower;
use crate::telemetry::Current;
use crate::telemetry::DischargeTemperature;
use crate::telemetry::EmergencyHeat;
use crate::telemetry::ErrorValue;
use crate::telemetry::EvaporationPressure;
use crate::telemetry::HeatPumpConsumed;
use crate::telemetry::HeatPumpPower;
use crate::telemetry::HeatingGenerated;
use crate::telemetry::HeatingPower;
use crate::telemetry::IndoorExchangerTemperature;
use crate::telemetry::IndoorExpansionValve;
use crate::telemetry::IndoorGasPipeTemperature;
use crate::telemetry::ModbusAddress;
use crate::telemetry::OutdoorExchangerTemperature;
use crate::telemetry::OutdoorExpansionValve;
use crate::telemetry::OutdoorTemperature;
use crate::telemetry::PhotovoltaicPower;
use crate::telemetry::Produced;
use crate::telemetry::ResistorHeatingConsumed;
use crate::telemetry::ReturnTemperature;
use crate::telemetry::RoomTemperature;
use crate::telemetry::Setpoint;
use crate::telemetry::T1T2Input;
use crate::telemetry::UnitError;
use crate::telemetry::WorkTemperature;
use crate::telemetry::flag;
use crate::units::CentiMegapascal;
use crate::units::DeciAmpere;
use crate::units::DeciCelsius;
use crate::units::KilowattHours;
use crate::units::Pulses;
use crate::units::Watts;

outcrop::strata! {
    pub(crate) struct Aidoo {
        #[strata(bank)]
        registers: [u16; 101] = [0; 101],

        AvailableModes = register::AvailableModes => |r| AvailableModes::from(r),
        AvailableSpeeds = register::AvailableSpeeds => |r| AvailableSpeeds::from(r),
        AvailableLouvers = register::AvailableLouvers => |r| AvailableLouvers::from(r),
        CoolLimits = (register::CoolMax, register::CoolMin) => |max, min| CoolLimits(Limits::new(max.0, min.0)),
        HeatLimits = (register::HeatMax, register::HeatMin) => |max, min| HeatLimits(Limits::new(max.0, min.0)),
        AutoLimits = (register::AutoMax, register::AutoMin) => |max, min| AutoLimits(Limits::new(max.0, min.0)),
        FanLimits = (register::FanMax, register::FanMin) => |max, min| FanLimits(Limits::new(max.0, min.0)),
        DryLimits = (register::DryMax, register::DryMin) => |max, min| DryLimits(Limits::new(max.0, min.0)),

        Option<Power> = register::Power => Power::new,
        Option<Mode> = (register::Mode, AvailableModes) => Mode::new,
        Option<FanSpeed> = register::FanSpeed => FanSpeed::new,
        Option<NumericFanSpeed> = register::NumericFanSpeed => NumericFanSpeed::new,
        VerticalLouvers = (register::VerticalLouvers, AvailableLouvers) => VerticalLouvers::new,
        HorizontalLouvers = (register::HorizontalLouvers, AvailableLouvers) => HorizontalLouvers::new,
        setpoint_range: SetpointRange = (
            Option<Mode>,
            CoolLimits,
            HeatLimits,
            AutoLimits,
            FanLimits,
            DryLimits,
        ) => setpoint_range,

        Setpoint = register::Setpoint => |r| Setpoint(DeciCelsius::from(r.0)),
        Option<RoomTemperature> = register::RoomTemperature => RoomTemperature::new,
        OutdoorTemperature = register::OutdoorTemperature => |r| OutdoorTemperature(DeciCelsius::from(r.0)),
        ReturnTemperature = register::ReturnTemperature => |r| ReturnTemperature(DeciCelsius::from(r.0)),
        IndoorExchangerTemperature = register::IndoorExchangerTemperature => |r| IndoorExchangerTemperature(DeciCelsius::from(r.0)),
        IndoorGasPipeTemperature = register::IndoorGasPipeTemperature => |r| IndoorGasPipeTemperature(DeciCelsius::from(r.0)),
        OutdoorExchangerTemperature = register::OutdoorExchangerTemperature => |r| OutdoorExchangerTemperature(DeciCelsius::from(r.0)),
        DischargeTemperature = register::DischargeTemperature => |r| DischargeTemperature(DeciCelsius::from(r.0)),
        WorkTemperature = register::WorkTemperature => |r| WorkTemperature(DeciCelsius::from(r.0)),
        OutdoorExpansionValve = register::OutdoorExpansionValve => |r| OutdoorExpansionValve(Pulses::from(r.0)),
        IndoorExpansionValve = register::IndoorExpansionValve => |r| IndoorExpansionValve(Pulses::from(r.0)),
        EvaporationPressure = register::EvaporationPressure => |r| EvaporationPressure(CentiMegapascal::from(r.0)),
        CondensationPressure = register::CondensationPressure => |r| CondensationPressure(CentiMegapascal::from(r.0)),
        Current = register::Current => |r| Current(DeciAmpere::from(r.0)),
        HeatingPower = register::HeatingPower => |r| HeatingPower(Watts::from(r.0)),
        CoolingPower = register::CoolingPower => |r| CoolingPower(Watts::from(r.0)),
        PhotovoltaicPower = register::PhotovoltaicPower => |r| PhotovoltaicPower(Watts::from(r.0)),
        HeatPumpPower = register::HeatPumpPower => |r| HeatPumpPower(Watts::from(r.0)),
        BuildingPower = register::BuildingPower => |r| BuildingPower(Watts::from(r.0)),

        HeatingGenerated = (register::HeatingGeneratedLow, register::HeatingGeneratedHigh) => |low, high| HeatingGenerated(KilowattHours::new(low.0, high.0)),
        CoolingGenerated = (register::CoolingGeneratedLow, register::CoolingGeneratedHigh) => |low, high| CoolingGenerated(KilowattHours::new(low.0, high.0)),
        Produced = (register::ProducedLow, register::ProducedHigh) => |low, high| Produced(KilowattHours::new(low.0, high.0)),
        HeatPumpConsumed = (register::HeatPumpConsumedLow, register::HeatPumpConsumedHigh) => |low, high| HeatPumpConsumed(KilowattHours::new(low.0, high.0)),
        ResistorHeatingConsumed = (register::ResistorHeatingConsumedLow, register::ResistorHeatingConsumedHigh) => |low, high| ResistorHeatingConsumed(KilowattHours::new(low.0, high.0)),
        CompressorHeatingConsumed = (register::CompressorHeatingConsumedLow, register::CompressorHeatingConsumedHigh) => |low, high| CompressorHeatingConsumed(KilowattHours::new(low.0, high.0)),
        CompressorCoolingConsumed = (register::CompressorCoolingConsumedLow, register::CompressorCoolingConsumedHigh) => |low, high| CompressorCoolingConsumed(KilowattHours::new(low.0, high.0)),
        Consumed = (register::ConsumedLow, register::ConsumedHigh) => |low, high| Consumed(KilowattHours::new(low.0, high.0)),

        Option<UnitError> = (register::UnitErrorFirst, register::UnitErrorSecond) => UnitError::new,
        Option<ErrorValue> = register::ErrorValue => ErrorValue::new,
        Option<EmergencyHeat> = register::EmergencyHeat => |r| flag(r.0).map(EmergencyHeat),
        Option<T1T2Input> = register::T1T2Input => |r| flag(r.0).map(T1T2Input),
        ModbusAddress = register::ModbusAddress => |r| ModbusAddress(r.0),
        Option<BaudRate> = register::BaudRate => BaudRate::new,
        Option<Parity> = register::Parity => Parity::new,

    }
}
