//! What a user controls: power, mode, fan speed and louvers.
//!
//! A value the map does not list, or one the unit's capability flags rule out,
//! decodes to `None`.

use crate::capabilities::AvailableLouvers;
use crate::capabilities::AvailableModes;
use crate::register;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Power {
    Off,
    On,
}

impl Power {
    pub const fn new(register: &register::Power) -> Option<Self> {
        match register.0 {
            0 => Some(Self::Off),
            1 => Some(Self::On),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Auto,
    Cool,
    Heat,
    Fan,
    Dry,
}

impl Mode {
    pub const ALL: [Self; 5] = [Self::Auto, Self::Cool, Self::Heat, Self::Fan, Self::Dry];

    /// Position of the mode in [`AvailableModes`].
    pub const fn bit(self) -> u16 {
        match self {
            Self::Auto => 0,
            Self::Cool => 1,
            Self::Heat => 2,
            Self::Fan => 3,
            Self::Dry => 4,
        }
    }

    /// The current mode, if the unit lists it among its available modes.
    pub fn new(register: &register::Mode, available: &AvailableModes) -> Option<Self> {
        let mode = match register.0 {
            1 => Self::Auto,
            2 => Self::Cool,
            3 => Self::Heat,
            4 => Self::Fan,
            5 => Self::Dry,
            _ => return None,
        };
        Some(mode).filter(|mode| available.supports(*mode))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanSpeed {
    Auto,
    Silent,
    Low,
    Medium,
    High,
}

impl FanSpeed {
    pub const fn new(register: &register::FanSpeed) -> Option<Self> {
        match register.0 {
            0 => Some(Self::Auto),
            25 => Some(Self::Silent),
            50 => Some(Self::Low),
            75 => Some(Self::Medium),
            100 => Some(Self::High),
            _ => None,
        }
    }
}

/// The fan speed again, encoded as a step instead of a percentage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NumericFanSpeed(FanSpeed);

impl NumericFanSpeed {
    pub const fn new(register: &register::NumericFanSpeed) -> Option<Self> {
        let speed = match register.0 {
            0 => FanSpeed::Auto,
            1 => FanSpeed::Silent,
            2 => FanSpeed::Low,
            3 => FanSpeed::Medium,
            4 => FanSpeed::High,
            _ => return None,
        };
        Some(Self(speed))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Louver {
    Position(u8),
    Auto,
    Swing,
    Swirl,
}

/// Four louvers, one per nibble, starting at the least significant.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Louvers([Option<Louver>; 4]);

impl Louvers {
    /// Decodes every nibble of `word`, keeping the louvers `supports` accepts.
    fn new(word: u16, supports: impl Fn(Louver) -> bool) -> Self {
        let [high, low] = word.to_be_bytes();
        let nibbles = [low & 0x0F, low >> 4, high & 0x0F, high >> 4];
        Self(nibbles.map(|nibble| {
            let louver = match nibble {
                0..=7 => Louver::Position(nibble),
                8 => Louver::Auto,
                9 => Louver::Swing,
                10 => Louver::Swirl,
                _ => return None,
            };
            Some(louver).filter(|louver| supports(*louver))
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerticalLouvers(Louvers);

impl VerticalLouvers {
    pub fn new(register: &register::VerticalLouvers, available: &AvailableLouvers) -> Self {
        Self(Louvers::new(register.0, |louver| match louver {
            Louver::Position(_) => true,
            Louver::Auto => available.auto_vertical,
            Louver::Swing => available.swing_vertical,
            Louver::Swirl => available.swirl,
        }))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HorizontalLouvers(Louvers);

impl HorizontalLouvers {
    /// The map has no flag for automatic horizontal louvers, so it is kept.
    pub fn new(register: &register::HorizontalLouvers, available: &AvailableLouvers) -> Self {
        Self(Louvers::new(register.0, |louver| match louver {
            Louver::Position(_) | Louver::Auto => true,
            Louver::Swing => available.swing_horizontal,
            Louver::Swirl => available.swirl,
        }))
    }
}
