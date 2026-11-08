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
use crate::register::Block;
use crate::register::Register;
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

macro_rules! declare {
    ({ $($signals:tt)* } ; $($field:ident: $alias:ident = $address:literal,)*) => {
        outcrop::strata! {
            pub(crate) struct Aidoo {
                $($field: register::$alias = Register(0),)*
                $($signals)*
                Block<'_> => |block, sink| {
                    for (address, &word) in (block.start..=u16::MAX).zip(block.words) {
                        match address {
                            $($address => sink.feed(Register::<$address>(word)),)*
                            _ => {}
                        }
                    }
                },
            }
        }
    };
}

register::table!(declare {
            AvailableModes = register::AvailableModes => |r| AvailableModes::from(r),
            AvailableSpeeds = register::AvailableSpeeds => |r| AvailableSpeeds::from(r),
            AvailableLouvers = register::AvailableLouvers => |r| AvailableLouvers::from(r),
            Option<Power> = register::Power => Power::new,
            Setpoint = register::Setpoint => |r| Setpoint::from(r),
            Option<RoomTemperature> = register::RoomTemperature => RoomTemperature::new,
            Option<Mode> = (register::Mode, AvailableModes) => Mode::new,
            Option<FanSpeed> = register::FanSpeed => FanSpeed::new,
            VerticalLouvers = (register::VerticalLouvers, AvailableLouvers) => VerticalLouvers::new,
            HorizontalLouvers = (register::HorizontalLouvers, AvailableLouvers) => HorizontalLouvers::new,
            Option<UnitError> = (register::UnitErrorFirst, register::UnitErrorSecond) => UnitError::new,
            CoolLimits = (register::CoolMax, register::CoolMin) => CoolLimits::new,
            HeatLimits = (register::HeatMax, register::HeatMin) => HeatLimits::new,
            AutoLimits = (register::AutoMax, register::AutoMin) => AutoLimits::new,
            FanLimits = (register::FanMax, register::FanMin) => FanLimits::new,
            DryLimits = (register::DryMax, register::DryMin) => DryLimits::new,
            setpoint_range: SetpointRange = (
                Option<Mode>,
                CoolLimits,
                HeatLimits,
                AutoLimits,
                FanLimits,
                DryLimits,
            ) => setpoint_range,
            OutdoorTemperature = register::OutdoorTemperature => |r| OutdoorTemperature::from(r),
            ReturnTemperature = register::ReturnTemperature => |r| ReturnTemperature::from(r),
            IndoorExchangerTemperature = register::IndoorExchangerTemperature => |r| IndoorExchangerTemperature::from(r),
            IndoorGasPipeTemperature = register::IndoorGasPipeTemperature => |r| IndoorGasPipeTemperature::from(r),
            OutdoorExchangerTemperature = register::OutdoorExchangerTemperature => |r| OutdoorExchangerTemperature::from(r),
            DischargeTemperature = register::DischargeTemperature => |r| DischargeTemperature::from(r),
            OutdoorExpansionValve = register::OutdoorExpansionValve => |r| OutdoorExpansionValve::from(r),
            IndoorExpansionValve = register::IndoorExpansionValve => |r| IndoorExpansionValve::from(r),
            EvaporationPressure = register::EvaporationPressure => |r| EvaporationPressure::from(r),
            CondensationPressure = register::CondensationPressure => |r| CondensationPressure::from(r),
            Current = register::Current => |r| Current::from(r),
            WorkTemperature = register::WorkTemperature => |r| WorkTemperature::from(r),
            Option<NumericFanSpeed> = register::NumericFanSpeed => NumericFanSpeed::new,
            Option<ErrorValue> = register::ErrorValue => ErrorValue::new,
            ModbusAddress = register::ModbusAddress => |r| ModbusAddress::from(r),
            Option<BaudRate> = register::BaudRate => BaudRate::new,
            Option<Parity> = register::Parity => Parity::new,
            Option<EmergencyHeat> = register::EmergencyHeat => EmergencyHeat::new,
            Option<T1T2Input> = register::T1T2Input => T1T2Input::new,
            HeatingGenerated = (register::HeatingGeneratedLow, register::HeatingGeneratedHigh) => HeatingGenerated::new,
            HeatingPower = register::HeatingPower => |r| HeatingPower::from(r),
            CoolingGenerated = (register::CoolingGeneratedLow, register::CoolingGeneratedHigh) => CoolingGenerated::new,
            CoolingPower = register::CoolingPower => |r| CoolingPower::from(r),
            PhotovoltaicPower = register::PhotovoltaicPower => |r| PhotovoltaicPower::from(r),
            Produced = (register::ProducedLow, register::ProducedHigh) => Produced::new,
            HeatPumpConsumed = (register::HeatPumpConsumedLow, register::HeatPumpConsumedHigh) => HeatPumpConsumed::new,
            HeatPumpPower = register::HeatPumpPower => |r| HeatPumpPower::from(r),
            BuildingPower = register::BuildingPower => |r| BuildingPower::from(r),
            ResistorHeatingConsumed = (register::ResistorHeatingConsumedLow, register::ResistorHeatingConsumedHigh) => ResistorHeatingConsumed::new,
            CompressorHeatingConsumed = (register::CompressorHeatingConsumedLow, register::CompressorHeatingConsumedHigh) => CompressorHeatingConsumed::new,
            CompressorCoolingConsumed = (register::CompressorCoolingConsumedLow, register::CompressorCoolingConsumedHigh) => CompressorCoolingConsumed::new,
            Consumed = (register::ConsumedLow, register::ConsumedHigh) => Consumed::new,
});
