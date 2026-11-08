//! The raw registers, as read from the device, and the address of each.

use core::fmt;
use core::fmt::Debug;

/// A holding register as last read from the device, before any decoding.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct Register<const ADDRESS: u16>(pub(crate) u16);

impl<const ADDRESS: u16> Debug for Register<ADDRESS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Register<{ADDRESS}>({})", self.0)
    }
}

/// Consecutive registers starting at `start`, as returned by a read.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Block<'a> {
    pub(crate) start: u16,
    pub(crate) words: &'a [u16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Invalid(pub(crate) u16);

macro_rules! table {
    ($callback:ident $($arguments:tt)*) => {
        $callback! {
            $($arguments)*
            ;
            power: Power = 0,
            setpoint: Setpoint = 1,
            room_temperature: RoomTemperature = 2,
            mode: Mode = 3,
            fan_speed: FanSpeed = 4,
            vertical_louvers: VerticalLouvers = 5,
            horizontal_louvers: HorizontalLouvers = 6,
            unit_error_first: UnitErrorFirst = 7,
            unit_error_second: UnitErrorSecond = 8,
            available_modes: AvailableModes = 14,
            available_speeds: AvailableSpeeds = 15,
            available_louvers: AvailableLouvers = 16,
            cool_max: CoolMax = 17,
            cool_min: CoolMin = 18,
            heat_max: HeatMax = 19,
            heat_min: HeatMin = 20,
            auto_max: AutoMax = 21,
            auto_min: AutoMin = 22,
            fan_max: FanMax = 23,
            fan_min: FanMin = 24,
            dry_max: DryMax = 25,
            dry_min: DryMin = 26,
            outdoor_temperature: OutdoorTemperature = 35,
            return_temperature: ReturnTemperature = 36,
            indoor_exchanger_temperature: IndoorExchangerTemperature = 37,
            indoor_gas_pipe_temperature: IndoorGasPipeTemperature = 38,
            outdoor_exchanger_temperature: OutdoorExchangerTemperature = 39,
            discharge_temperature: DischargeTemperature = 40,
            outdoor_expansion_valve: OutdoorExpansionValve = 41,
            indoor_expansion_valve: IndoorExpansionValve = 42,
            evaporation_pressure: EvaporationPressure = 43,
            condensation_pressure: CondensationPressure = 44,
            current: Current = 45,
            work_temperature: WorkTemperature = 53,
            numeric_fan_speed: NumericFanSpeed = 54,
            error_value: ErrorValue = 55,
            modbus_address: ModbusAddress = 56,
            baud_rate: BaudRate = 57,
            parity: Parity = 58,
            emergency_heat: EmergencyHeat = 59,
            t1_t2_input: T1T2Input = 60,
            heating_generated_low: HeatingGeneratedLow = 73,
            heating_generated_high: HeatingGeneratedHigh = 74,
            heating_power: HeatingPower = 75,
            cooling_generated_low: CoolingGeneratedLow = 76,
            cooling_generated_high: CoolingGeneratedHigh = 77,
            cooling_power: CoolingPower = 78,
            photovoltaic_power: PhotovoltaicPower = 82,
            produced_low: ProducedLow = 83,
            produced_high: ProducedHigh = 84,
            heat_pump_consumed_low: HeatPumpConsumedLow = 85,
            heat_pump_consumed_high: HeatPumpConsumedHigh = 86,
            heat_pump_power: HeatPumpPower = 87,
            building_power: BuildingPower = 88,
            resistor_heating_consumed_low: ResistorHeatingConsumedLow = 89,
            resistor_heating_consumed_high: ResistorHeatingConsumedHigh = 90,
            compressor_heating_consumed_low: CompressorHeatingConsumedLow = 93,
            compressor_heating_consumed_high: CompressorHeatingConsumedHigh = 94,
            compressor_cooling_consumed_low: CompressorCoolingConsumedLow = 95,
            compressor_cooling_consumed_high: CompressorCoolingConsumedHigh = 96,
            consumed_low: ConsumedLow = 99,
            consumed_high: ConsumedHigh = 100,
        }
    };
}

pub(crate) use table;

macro_rules! aliases {
    (; $($field:ident: $alias:ident = $address:literal,)*) => {
        $(pub(crate) type $alias = Register<$address>;)*
    };
}

table!(aliases);
