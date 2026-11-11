//! What the unit supports: modes, fan speeds, louvers and setpoint limits.

use crate::control::Mode;
use crate::register;
use crate::units::DeciCelsius;
use core::fmt;
use core::fmt::Debug;

/// Fan speeds a unit may support, as listed in [`AvailableSpeeds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpeedStep {
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
    pub const ALL: [Self; 8] = [
        Self::Auto,
        Self::SuperLow,
        Self::Low,
        Self::MediumLow,
        Self::Medium,
        Self::MediumHigh,
        Self::High,
        Self::SuperHigh,
    ];

    pub const fn bit(self) -> u16 {
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
pub struct AvailableSpeeds(u16);

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
    pub const fn supports(self, step: SpeedStep) -> bool {
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
pub struct AvailableLouvers {
    pub auto_vertical: bool,
    pub swing_vertical: bool,
    pub swing_horizontal: bool,
    pub swirl: bool,
    pub vertical_positions: u8,
    pub horizontal_positions: u8,
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct AvailableModes(u16);

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
    pub const fn supports(self, mode: Mode) -> bool {
        self.0 & 1 << mode.bit() != 0
    }
}

impl From<&register::AvailableModes> for AvailableModes {
    fn from(register: &register::AvailableModes) -> Self {
        Self(register.0)
    }
}

/// Setpoint limits of one mode, read from its maximum and minimum registers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    min: DeciCelsius,
    max: DeciCelsius,
}

impl Limits {
    pub fn new(max: u16, min: u16) -> Self {
        Self {
            min: DeciCelsius::from(min),
            max: DeciCelsius::from(max),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoolLimits(pub Limits);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HeatLimits(pub Limits);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutoLimits(pub Limits);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FanLimits(pub Limits);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DryLimits(pub Limits);

/// Limits of the setpoint in the current mode, if there is one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetpointRange(Option<Limits>);

#[expect(
    clippy::ref_option,
    reason = "derived signals receive their dependencies by reference"
)]
pub fn setpoint_range(
    mode: &Option<Mode>,
    cool: &CoolLimits,
    heat: &HeatLimits,
    auto: &AutoLimits,
    fan: &FanLimits,
    dry: &DryLimits,
) -> SetpointRange {
    SetpointRange(mode.map(|mode| match mode {
        Mode::Auto => auto.0,
        Mode::Cool => cool.0,
        Mode::Heat => heat.0,
        Mode::Fan => fan.0,
        Mode::Dry => dry.0,
    }))
}
