//! What the unit supports: modes, fan speeds, louvers and setpoint limits.

use crate::control::Mode;
use crate::register;
use crate::register::Invalid;
use crate::units::DeciCelsius;
use core::fmt;
use core::fmt::Debug;

/// Fan speeds a unit may support, as listed in [`AvailableSpeeds`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SpeedStep {
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
    pub(crate) const ALL: [Self; 8] = [
        Self::Auto,
        Self::SuperLow,
        Self::Low,
        Self::MediumLow,
        Self::Medium,
        Self::MediumHigh,
        Self::High,
        Self::SuperHigh,
    ];

    pub(crate) const fn bit(self) -> u16 {
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
pub(crate) struct AvailableSpeeds(u16);

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
    pub(crate) const fn supports(self, step: SpeedStep) -> bool {
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
pub(crate) struct AvailableLouvers {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct AvailableModes(u16);

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
    pub(crate) const fn supports(self, mode: Mode) -> bool {
        self.0 & 1 << mode.bit() != 0
    }
}

impl From<&register::AvailableModes> for AvailableModes {
    fn from(register: &register::AvailableModes) -> Self {
        Self(register.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Limits {
    min: DeciCelsius,
    max: DeciCelsius,
}

/// Setpoint limits of one mode, read from its maximum and minimum registers.
macro_rules! limits {
    ($($name:ident = $max:ty, $min:ty;)*) => {$(
        #[derive(Debug, Clone, Copy, PartialEq, Eq)]
        pub(crate) struct $name(Limits);

        impl $name {
            pub(crate) fn new(max: &$max, min: &$min) -> Self {
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
pub(crate) struct SetpointRange(Option<Limits>);

pub(crate) fn setpoint_range(
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
