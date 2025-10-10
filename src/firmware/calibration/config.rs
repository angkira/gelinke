//! Calibration configuration (from iRPC)

use defmt::Format;

/// Calibration configuration received from iRPC
#[derive(Debug, Format, Clone, Copy)]
pub struct CalibrationConfig {
    /// Phases to run (bitmask)
    pub phases: u8,
    /// Maximum test current (A)
    pub max_current: f32,
    /// Maximum test velocity (rad/s)
    pub max_velocity: f32,
    /// Maximum position excursion (rad)
    pub max_position_range: f32,
    /// Timeout per phase (seconds)
    pub phase_timeout: f32,
    /// Return to home position
    pub return_home: bool,
    /// Home position (saved at start)
    pub home_position: f32,
}

impl CalibrationConfig {
    pub fn validate(&self) -> Result<(), &'static str> {
        if self.max_current <= 0.0 || self.max_current > 15.0 {
            return Err("Invalid max_current");
        }
        if self.max_velocity <= 0.0 || self.max_velocity > 10.0 {
            return Err("Invalid max_velocity");
        }
        if self.max_position_range <= 0.0 {
            return Err("Invalid max_position_range");
        }
        if self.phase_timeout <= 0.0 {
            return Err("Invalid phase_timeout");
        }
        Ok(())
    }

    pub fn phase_enabled(&self, phase: u8) -> bool {
        (self.phases & (1 << phase)) != 0
    }
}

/// Phase indices for bitmask
pub const PHASE_INERTIA: u8 = 0;
pub const PHASE_FRICTION: u8 = 1;
pub const PHASE_TORQUE_CONSTANT: u8 = 2;
pub const PHASE_DAMPING: u8 = 3;
pub const PHASE_VALIDATION: u8 = 4;
