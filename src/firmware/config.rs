pub const SYSCLK_HZ: u32 = 170_000_000;
pub const HEARTBEAT_PERIOD_SECS: u64 = 1;

/// Control method selection
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ControlMethod {
    /// Field-Oriented Control (FOC) for smooth, efficient operation
    Foc,
    /// Classic Step-Dir control for compatibility with step/dir interfaces
    StepDir,
}

impl Default for ControlMethod {
    fn default() -> Self {
        Self::Foc
    }
}

#[derive(Clone, Copy, Debug)]
pub struct MotorConfig {
    pub pole_pairs: u8,
    pub control_method: ControlMethod,
    /// Microstepping resolution for Step-Dir mode (1, 2, 4, 8, 16, 32, 64, 128, 256)
    pub microsteps: u16,
}

impl Default for MotorConfig {
    fn default() -> Self {
        Self {
            pole_pairs: 7,
            control_method: ControlMethod::Foc,
            microsteps: 16,
        }
    }
}

impl MotorConfig {
    pub const fn default() -> Self {
        Self {
            pole_pairs: 7,
            control_method: ControlMethod::Foc,
            microsteps: 16,
        }
    }
}

#[derive(Clone, Copy)]
pub struct EncoderConfig {
    pub resolution_bits: u8,
}

impl EncoderConfig {
    pub const fn tle5012b() -> Self {
        Self {
            resolution_bits: 15,
        }
    }
}
