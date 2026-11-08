//! The raw registers, as read from the device, and the address of each.

use outcrop::Slot;

/// A holding register as last read from the device, before any decoding.
pub(crate) struct Register<const ADDRESS: usize>(pub(crate) u16);

impl<const ADDRESS: usize> Slot for Register<ADDRESS> {
    type Item = u16;
    const INDEX: usize = ADDRESS;

    fn read(word: &u16) -> Self {
        Self(*word)
    }
}

pub(crate) type Power = Register<0>;
pub(crate) type Setpoint = Register<1>;
pub(crate) type RoomTemperature = Register<2>;
pub(crate) type Mode = Register<3>;
pub(crate) type FanSpeed = Register<4>;
pub(crate) type VerticalLouvers = Register<5>;
pub(crate) type HorizontalLouvers = Register<6>;
pub(crate) type UnitErrorFirst = Register<7>;
pub(crate) type UnitErrorSecond = Register<8>;
pub(crate) type AvailableModes = Register<14>;
pub(crate) type AvailableSpeeds = Register<15>;
pub(crate) type AvailableLouvers = Register<16>;
pub(crate) type CoolMax = Register<17>;
pub(crate) type CoolMin = Register<18>;
pub(crate) type HeatMax = Register<19>;
pub(crate) type HeatMin = Register<20>;
pub(crate) type AutoMax = Register<21>;
pub(crate) type AutoMin = Register<22>;
pub(crate) type FanMax = Register<23>;
pub(crate) type FanMin = Register<24>;
pub(crate) type DryMax = Register<25>;
pub(crate) type DryMin = Register<26>;
pub(crate) type OutdoorTemperature = Register<35>;
pub(crate) type ReturnTemperature = Register<36>;
pub(crate) type IndoorExchangerTemperature = Register<37>;
pub(crate) type IndoorGasPipeTemperature = Register<38>;
pub(crate) type OutdoorExchangerTemperature = Register<39>;
pub(crate) type DischargeTemperature = Register<40>;
pub(crate) type OutdoorExpansionValve = Register<41>;
pub(crate) type IndoorExpansionValve = Register<42>;
pub(crate) type EvaporationPressure = Register<43>;
pub(crate) type CondensationPressure = Register<44>;
pub(crate) type Current = Register<45>;
pub(crate) type WorkTemperature = Register<53>;
pub(crate) type NumericFanSpeed = Register<54>;
pub(crate) type ErrorValue = Register<55>;
pub(crate) type ModbusAddress = Register<56>;
pub(crate) type BaudRate = Register<57>;
pub(crate) type Parity = Register<58>;
pub(crate) type EmergencyHeat = Register<59>;
pub(crate) type T1T2Input = Register<60>;
pub(crate) type HeatingGeneratedLow = Register<73>;
pub(crate) type HeatingGeneratedHigh = Register<74>;
pub(crate) type HeatingPower = Register<75>;
pub(crate) type CoolingGeneratedLow = Register<76>;
pub(crate) type CoolingGeneratedHigh = Register<77>;
pub(crate) type CoolingPower = Register<78>;
pub(crate) type PhotovoltaicPower = Register<82>;
pub(crate) type ProducedLow = Register<83>;
pub(crate) type ProducedHigh = Register<84>;
pub(crate) type HeatPumpConsumedLow = Register<85>;
pub(crate) type HeatPumpConsumedHigh = Register<86>;
pub(crate) type HeatPumpPower = Register<87>;
pub(crate) type BuildingPower = Register<88>;
pub(crate) type ResistorHeatingConsumedLow = Register<89>;
pub(crate) type ResistorHeatingConsumedHigh = Register<90>;
pub(crate) type CompressorHeatingConsumedLow = Register<93>;
pub(crate) type CompressorHeatingConsumedHigh = Register<94>;
pub(crate) type CompressorCoolingConsumedLow = Register<95>;
pub(crate) type CompressorCoolingConsumedHigh = Register<96>;
pub(crate) type ConsumedLow = Register<99>;
pub(crate) type ConsumedHigh = Register<100>;
