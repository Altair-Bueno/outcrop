//! What a user controls: power, mode, fan speed and louvers.

use crate::register;
use crate::register::Invalid;
use core::fmt::Debug;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Power {
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
pub(crate) enum Mode {
    Auto,
    Cool,
    Heat,
    Fan,
    Dry,
}

impl Mode {
    pub(crate) const ALL: [Self; 5] = [Self::Auto, Self::Cool, Self::Heat, Self::Fan, Self::Dry];

    /// Position of the mode in [`AvailableModes`].
    pub(crate) const fn bit(self) -> u16 {
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
pub(crate) enum FanSpeed {
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
pub(crate) struct NumericFanSpeed(FanSpeed);

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Louver {
    Position(u8),
    Auto,
    Swing,
    Swirl,
}

/// Four louvers, one per nibble, starting at the least significant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Louvers([Louver; 4]);

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
pub(crate) struct VerticalLouvers(Louvers);

impl TryFrom<&register::VerticalLouvers> for VerticalLouvers {
    type Error = Invalid;

    fn try_from(register: &register::VerticalLouvers) -> Result<Self, Invalid> {
        Louvers::try_from(register.0).map(Self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct HorizontalLouvers(Louvers);

impl TryFrom<&register::HorizontalLouvers> for HorizontalLouvers {
    type Error = Invalid;

    fn try_from(register: &register::HorizontalLouvers) -> Result<Self, Invalid> {
        Louvers::try_from(register.0).map(Self)
    }
}
