//! Read side of an Airzone Aidoo driving a direct expansion unit over Modbus.
//!
//! Register map: Aidoo Modbus manual, "Registers for Direct Expansion units".
#![expect(
    clippy::trivially_copy_pass_by_ref,
    reason = "derived signals receive their dependencies by reference"
)]

use core::fmt;
use core::fmt::Debug;

use outcrop::Formation;
use outcrop::Visit;

/// A holding register as last read from the device, before any decoding.
#[derive(Clone, Copy, PartialEq, Eq)]
struct Register<const ADDRESS: u16>(u16);

impl<const ADDRESS: u16> Debug for Register<ADDRESS> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Register<{ADDRESS}>({})", self.0)
    }
}

/// Consecutive registers starting at `start`, as returned by a read.
#[derive(Debug, Clone, Copy)]
struct Block<'a> {
    start: u16,
    words: &'a [u16],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Invalid(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeciCelsius(i16);

impl From<u16> for DeciCelsius {
    fn from(word: u16) -> Self {
        Self(word.cast_signed())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CentiMegapascal(u16);

impl From<u16> for CentiMegapascal {
    fn from(word: u16) -> Self {
        Self(word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DeciAmpere(u16);

impl From<u16> for DeciAmpere {
    fn from(word: u16) -> Self {
        Self(word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Watts(u16);

impl From<u16> for Watts {
    fn from(word: u16) -> Self {
        Self(word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pulses(u16);

impl From<u16> for Pulses {
    fn from(word: u16) -> Self {
        Self(word)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct KilowattHours(u32);

impl KilowattHours {
    fn new(low: u16, high: u16) -> Self {
        Self(u32::from(high) << 16 | u32::from(low))
    }
}

/// Signals decoded from a single register into a unit.
macro_rules! measurements {
    ($($name:ident($unit:ident) = $register:ty;)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct $name($unit);

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
        struct $name(KilowattHours);

        impl $name {
            fn new(low: &$low, high: &$high) -> Self {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Power {
    Off,
    On,
}

impl TryFrom<&register::Power> for Power {
    type Error = Invalid;

    fn try_from(register: &register::Power) -> Result<Self, Invalid> {
        match register.0 {
            0 => Ok(Self::Off),
            1 => Ok(Self::On),
            other => Err(Invalid(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Auto,
    Cool,
    Heat,
    Fan,
    Dry,
}

impl Mode {
    const ALL: [Self; 5] = [Self::Auto, Self::Cool, Self::Heat, Self::Fan, Self::Dry];

    /// Position of the mode in [`AvailableModes`].
    const fn bit(self) -> u16 {
        match self {
            Self::Auto => 0,
            Self::Cool => 1,
            Self::Heat => 2,
            Self::Fan => 3,
            Self::Dry => 4,
        }
    }
}

impl TryFrom<&register::Mode> for Mode {
    type Error = Invalid;

    fn try_from(register: &register::Mode) -> Result<Self, Invalid> {
        match register.0 {
            1 => Ok(Self::Auto),
            2 => Ok(Self::Cool),
            3 => Ok(Self::Heat),
            4 => Ok(Self::Fan),
            5 => Ok(Self::Dry),
            other => Err(Invalid(other)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FanSpeed {
    Auto,
    Silent,
    Low,
    Medium,
    High,
}

impl TryFrom<&register::FanSpeed> for FanSpeed {
    type Error = Invalid;

    fn try_from(register: &register::FanSpeed) -> Result<Self, Invalid> {
        match register.0 {
            0 => Ok(Self::Auto),
            25 => Ok(Self::Silent),
            50 => Ok(Self::Low),
            75 => Ok(Self::Medium),
            100 => Ok(Self::High),
            other => Err(Invalid(other)),
        }
    }
}

/// The fan speed again, encoded as a step instead of a percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NumericFanSpeed(FanSpeed);

impl TryFrom<&register::NumericFanSpeed> for NumericFanSpeed {
    type Error = Invalid;

    fn try_from(register: &register::NumericFanSpeed) -> Result<Self, Invalid> {
        match register.0 {
            0 => Ok(Self(FanSpeed::Auto)),
            1 => Ok(Self(FanSpeed::Silent)),
            2 => Ok(Self(FanSpeed::Low)),
            3 => Ok(Self(FanSpeed::Medium)),
            4 => Ok(Self(FanSpeed::High)),
            other => Err(Invalid(other)),
        }
    }
}

/// Fan speeds a unit may support, as listed in [`AvailableSpeeds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SpeedStep {
    Auto,
    SuperLow,
    Low,
    MediumLow,
    Medium,
    MediumHigh,
    High,
    SuperHigh,
}

impl SpeedStep {
    const ALL: [Self; 8] = [
        Self::Auto,
        Self::SuperLow,
        Self::Low,
        Self::MediumLow,
        Self::Medium,
        Self::MediumHigh,
        Self::High,
        Self::SuperHigh,
    ];

    const fn bit(self) -> u16 {
        match self {
            Self::Auto => 0,
            Self::SuperLow => 1,
            Self::Low => 2,
            Self::MediumLow => 3,
            Self::Medium => 4,
            Self::MediumHigh => 5,
            Self::High => 6,
            Self::SuperHigh => 7,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct AvailableSpeeds(u16);

impl Debug for AvailableSpeeds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AvailableSpeeds")
            .field(&fmt::from_fn(|f| {
                f.debug_set()
                    .entries(
                        SpeedStep::ALL
                            .into_iter()
                            .filter(|step| self.supports(*step)),
                    )
                    .finish()
            }))
            .finish()
    }
}

impl AvailableSpeeds {
    const fn supports(self, step: SpeedStep) -> bool {
        self.0 & 1 << step.bit() != 0
    }
}

impl From<&register::AvailableSpeeds> for AvailableSpeeds {
    fn from(register: &register::AvailableSpeeds) -> Self {
        Self(register.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent bit of the register"
)]
struct AvailableLouvers {
    auto_vertical: bool,
    swing_vertical: bool,
    swing_horizontal: bool,
    swirl: bool,
    vertical_positions: u8,
    horizontal_positions: u8,
}

impl From<&register::AvailableLouvers> for AvailableLouvers {
    fn from(register: &register::AvailableLouvers) -> Self {
        let word = register.0;
        let [high, _] = word.to_be_bytes();
        Self {
            auto_vertical: word & 1 != 0,
            swing_vertical: word & 1 << 3 != 0,
            swing_horizontal: word & 1 << 4 != 0,
            swirl: word & 1 << 5 != 0,
            vertical_positions: high & 0x0F,
            horizontal_positions: high >> 4,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BaudRate(u32);

impl TryFrom<&register::BaudRate> for BaudRate {
    type Error = Invalid;

    fn try_from(register: &register::BaudRate) -> Result<Self, Invalid> {
        let rate = match register.0 {
            0 => 100,
            1 => 300,
            2 => 500,
            3 => 1200,
            4 => 2400,
            5 => 4800,
            6 => 7800,
            7 => 9600,
            8 => 19200,
            9 => 57600,
            10 => 115_200,
            other => return Err(Invalid(other)),
        };
        Ok(Self(rate))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Parity {
    None,
    Odd,
    Even,
}

impl TryFrom<&register::Parity> for Parity {
    type Error = Invalid;

    fn try_from(register: &register::Parity) -> Result<Self, Invalid> {
        match register.0 {
            0 => Ok(Self::None),
            1 => Ok(Self::Odd),
            2 => Ok(Self::Even),
            other => Err(Invalid(other)),
        }
    }
}

/// Signals decoded from a register holding 0 (inactive) or 1 (active).
macro_rules! flags {
    ($($name:ident = $register:ty;)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct $name(bool);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Louver {
    Position(u8),
    Auto,
    Swing,
    Swirl,
}

/// Four louvers, one per nibble, starting at the least significant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Louvers([Louver; 4]);

impl TryFrom<u16> for Louvers {
    type Error = Invalid;

    fn try_from(word: u16) -> Result<Self, Invalid> {
        let [high, low] = word.to_be_bytes();
        let nibbles = [low & 0x0F, low >> 4, high & 0x0F, high >> 4];
        let mut louvers = [Louver::Auto; 4];
        for (louver, nibble) in louvers.iter_mut().zip(nibbles) {
            *louver = match nibble {
                0..=7 => Louver::Position(nibble),
                8 => Louver::Auto,
                9 => Louver::Swing,
                10 => Louver::Swirl,
                _ => return Err(Invalid(word)),
            };
        }
        Ok(Self(louvers))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VerticalLouvers(Louvers);

impl TryFrom<&register::VerticalLouvers> for VerticalLouvers {
    type Error = Invalid;

    fn try_from(register: &register::VerticalLouvers) -> Result<Self, Invalid> {
        Louvers::try_from(register.0).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct HorizontalLouvers(Louvers);

impl TryFrom<&register::HorizontalLouvers> for HorizontalLouvers {
    type Error = Invalid;

    fn try_from(register: &register::HorizontalLouvers) -> Result<Self, Invalid> {
        Louvers::try_from(register.0).map(Self)
    }
}

/// Error code of the unit, as four ASCII characters.
#[derive(Clone, Copy, PartialEq, Eq)]
struct UnitError([u8; 4]);

impl UnitError {
    fn new(first: &register::UnitErrorFirst, second: &register::UnitErrorSecond) -> Self {
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

#[derive(Clone, Copy, PartialEq, Eq)]
struct AvailableModes(u16);

impl Debug for AvailableModes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("AvailableModes")
            .field(&fmt::from_fn(|f| {
                f.debug_set()
                    .entries(Mode::ALL.into_iter().filter(|mode| self.supports(*mode)))
                    .finish()
            }))
            .finish()
    }
}

impl AvailableModes {
    const fn supports(self, mode: Mode) -> bool {
        self.0 & 1 << mode.bit() != 0
    }
}

impl From<&register::AvailableModes> for AvailableModes {
    fn from(register: &register::AvailableModes) -> Self {
        Self(register.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Limits {
    min: DeciCelsius,
    max: DeciCelsius,
}

/// Setpoint limits of one mode, read from its maximum and minimum registers.
macro_rules! limits {
    ($($name:ident = $max:ty, $min:ty;)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        struct $name(Limits);

        impl $name {
            fn new(max: &$max, min: &$min) -> Self {
                Self(Limits {
                    min: DeciCelsius::from(min.0),
                    max: DeciCelsius::from(max.0),
                })
            }
        }
    )*};
}

limits! {
    CoolLimits = register::CoolMax, register::CoolMin;
    HeatLimits = register::HeatMax, register::HeatMin;
    AutoLimits = register::AutoMax, register::AutoMin;
    FanLimits = register::FanMax, register::FanMin;
    DryLimits = register::DryMax, register::DryMin;
}

/// Limits of the setpoint in the current mode, if the unit supports it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SetpointRange(Option<Limits>);

fn setpoint_range(
    mode: &Result<Mode, Invalid>,
    available: &AvailableModes,
    cool: &CoolLimits,
    heat: &HeatLimits,
    auto: &AutoLimits,
    fan: &FanLimits,
    dry: &DryLimits,
) -> SetpointRange {
    SetpointRange(
        mode.ok()
            .filter(|mode| available.supports(*mode))
            .map(|mode| match mode {
                Mode::Auto => auto.0,
                Mode::Cool => cool.0,
                Mode::Heat => heat.0,
                Mode::Fan => fan.0,
                Mode::Dry => dry.0,
            }),
    )
}

/// Declares a formation with one input per register, the given signals above
/// them, and a router on top feeding a [`Block`] into the registers.
///
/// Each register is also named in a `register` module, so `register::Mode` is
/// the raw word that decodes into a `Mode`.
macro_rules! registers {
    (
        struct $name:ident {
            $($field:ident: $alias:ident = $address:literal,)*
        }
        $($signals:tt)*
    ) => {
        #[expect(
            clippy::redundant_pub_crate,
            reason = "`unreachable_pub` asks for the opposite inside private modules"
        )]
        mod register {
            use super::Register;

            $(pub(super) type $alias = Register<$address>;)*
        }

        outcrop::strata! {
            struct $name {
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

registers! {
    struct Aidoo {
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

    Result<Power, Invalid> = register::Power => |r| Power::try_from(r),
    Setpoint = register::Setpoint => |r| Setpoint::from(r),
    RoomTemperature = register::RoomTemperature => |r| RoomTemperature::from(r),
    Result<Mode, Invalid> = register::Mode => |r| Mode::try_from(r),
    Result<FanSpeed, Invalid> = register::FanSpeed => |r| FanSpeed::try_from(r),
    Result<VerticalLouvers, Invalid> = register::VerticalLouvers => |r| VerticalLouvers::try_from(r),
    Result<HorizontalLouvers, Invalid> = register::HorizontalLouvers => |r| HorizontalLouvers::try_from(r),
    UnitError = (register::UnitErrorFirst, register::UnitErrorSecond) => UnitError::new,
    AvailableModes = register::AvailableModes => |r| AvailableModes::from(r),
    AvailableSpeeds = register::AvailableSpeeds => |r| AvailableSpeeds::from(r),
    AvailableLouvers = register::AvailableLouvers => |r| AvailableLouvers::from(r),
    CoolLimits = (register::CoolMax, register::CoolMin) => CoolLimits::new,
    HeatLimits = (register::HeatMax, register::HeatMin) => HeatLimits::new,
    AutoLimits = (register::AutoMax, register::AutoMin) => AutoLimits::new,
    FanLimits = (register::FanMax, register::FanMin) => FanLimits::new,
    DryLimits = (register::DryMax, register::DryMin) => DryLimits::new,
    setpoint_range: SetpointRange = (
        Result<Mode, Invalid>,
        AvailableModes,
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
    Result<NumericFanSpeed, Invalid> = register::NumericFanSpeed => |r| NumericFanSpeed::try_from(r),
    ErrorValue = register::ErrorValue => |r| ErrorValue::from(r),
    ModbusAddress = register::ModbusAddress => |r| ModbusAddress::from(r),
    Result<BaudRate, Invalid> = register::BaudRate => |r| BaudRate::try_from(r),
    Result<Parity, Invalid> = register::Parity => |r| Parity::try_from(r),
    Result<EmergencyHeat, Invalid> = register::EmergencyHeat => |r| EmergencyHeat::try_from(r),
    Result<T1T2Input, Invalid> = register::T1T2Input => |r| T1T2Input::try_from(r),
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
}

struct Print;

impl<T: Debug> Visit<T> for Print {
    #[expect(clippy::print_stdout, reason = "the example shows what a poll reports")]
    fn visit(&mut self, value: &T) {
        println!("  {value:?}");
    }
}

#[expect(clippy::print_stdout, reason = "the example shows what a poll reports")]
fn main() {
    let mut aidoo = Formation::from(Aidoo::default());

    // Startup: read every block once before the first poll, so the zeros
    // the registers start with are never reported.
    let status = [1, 230, 215, 2, 50, 0x0098, 0x0008, 0, 0];
    let capabilities = [
        0b1_1111, 0b111_1101, 0x1131, 300, 180, 300, 160, 300, 160, 300, 160, 300, 160,
    ];
    let sensors = [0xFFE2_u16, 205, 120, 110, 450, 750, 120, 150, 98, 245, 70];
    aidoo.feed(&Block {
        start: 0,
        words: &status,
    });
    aidoo.feed(&Block {
        start: 14,
        words: &capabilities,
    });
    aidoo.feed(&Block {
        start: 35,
        words: &sensors,
    });
    aidoo.feed(&Block {
        start: 53,
        words: &[220, 2, 0, 1, 8, 2, 0, 0],
    });
    aidoo.feed(&Block {
        start: 73,
        words: &[0x1215, 0x0001, 0x1CAC],
    });

    println!("first poll:");
    aidoo.poll(&mut Print);

    // Someone switches the unit to heating at the wall.
    aidoo.feed(&Block {
        start: 0,
        words: &[1, 210, 215, 3, 50],
    });

    println!("after switching to heat:");
    aidoo.poll(&mut Print);

    println!(
        "setpoint range: {:?}",
        aidoo.get::<SetpointRange, _>().into_inner()
    );
}
