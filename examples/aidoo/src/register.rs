//! The raw registers, as read from the device, and the address of each.

use outcrop::Slot;

/// A holding register as last read from the device, before any decoding.
pub struct Register<const ADDRESS: usize>(pub u16);

impl<const ADDRESS: usize> Slot for Register<ADDRESS> {
    type Item = u16;
    const INDEX: usize = ADDRESS;

    fn read(word: &u16) -> Self {
        Self(*word)
    }
}

pub type Power = Register<0>;
pub type Setpoint = Register<1>;
pub type RoomTemperature = Register<2>;
pub type Mode = Register<3>;
pub type FanSpeed = Register<4>;
pub type VerticalLouvers = Register<5>;
pub type HorizontalLouvers = Register<6>;
pub type UnitErrorFirst = Register<7>;
pub type UnitErrorSecond = Register<8>;
pub type AvailableModes = Register<14>;
pub type AvailableSpeeds = Register<15>;
pub type AvailableLouvers = Register<16>;
pub type CoolMax = Register<17>;
pub type CoolMin = Register<18>;
pub type HeatMax = Register<19>;
pub type HeatMin = Register<20>;
pub type AutoMax = Register<21>;
pub type AutoMin = Register<22>;
pub type FanMax = Register<23>;
pub type FanMin = Register<24>;
pub type DryMax = Register<25>;
pub type DryMin = Register<26>;
pub type OutdoorTemperature = Register<35>;
pub type ReturnTemperature = Register<36>;
pub type IndoorExchangerTemperature = Register<37>;
pub type IndoorGasPipeTemperature = Register<38>;
pub type OutdoorExchangerTemperature = Register<39>;
pub type DischargeTemperature = Register<40>;
pub type OutdoorExpansionValve = Register<41>;
pub type IndoorExpansionValve = Register<42>;
pub type EvaporationPressure = Register<43>;
pub type CondensationPressure = Register<44>;
pub type Current = Register<45>;
pub type WorkTemperature = Register<53>;
pub type NumericFanSpeed = Register<54>;
pub type ErrorValue = Register<55>;
pub type ModbusAddress = Register<56>;
pub type BaudRate = Register<57>;
pub type Parity = Register<58>;
pub type EmergencyHeat = Register<59>;
pub type T1T2Input = Register<60>;
pub type HeatingGeneratedLow = Register<73>;
pub type HeatingGeneratedHigh = Register<74>;
pub type HeatingPower = Register<75>;
pub type CoolingGeneratedLow = Register<76>;
pub type CoolingGeneratedHigh = Register<77>;
pub type CoolingPower = Register<78>;
pub type PhotovoltaicPower = Register<82>;
pub type ProducedLow = Register<83>;
pub type ProducedHigh = Register<84>;
pub type HeatPumpConsumedLow = Register<85>;
pub type HeatPumpConsumedHigh = Register<86>;
pub type HeatPumpPower = Register<87>;
pub type BuildingPower = Register<88>;
pub type ResistorHeatingConsumedLow = Register<89>;
pub type ResistorHeatingConsumedHigh = Register<90>;
pub type CompressorHeatingConsumedLow = Register<93>;
pub type CompressorHeatingConsumedHigh = Register<94>;
pub type CompressorCoolingConsumedLow = Register<95>;
pub type CompressorCoolingConsumedHigh = Register<96>;
pub type ConsumedLow = Register<99>;
pub type ConsumedHigh = Register<100>;
