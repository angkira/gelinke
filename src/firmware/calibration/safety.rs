//! Safety monitoring for calibration tests

use defmt::Format;
use super::config::CalibrationConfig;
use super::types::{ErrorCode, MeasurementSample};

#[derive(Debug, Format)]
pub struct SafetyMonitor {
    config: CalibrationConfig,
    start_time: f32,
    phase_start_time: f32,
    home_position: f32,
}

impl SafetyMonitor {
    pub fn new(config: CalibrationConfig, current_position: f32, current_time: f32) -> Self {
        Self {
            config,
            start_time: current_time,
            phase_start_time: current_time,
            home_position: current_position,
        }
    }

    pub fn reset_phase_timer(&mut self, current_time: f32) {
        self.phase_start_time = current_time;
    }

    /// Check if measurement violates safety limits
    pub fn check_safety(&self, sample: &MeasurementSample) -> Result<(), ErrorCode> {
        // Position limit
        let position_error = (sample.position - self.home_position).abs();
        if position_error > self.config.max_position_range {
            defmt::error!("Position limit exceeded: {} rad", position_error);
            return Err(ErrorCode::PositionLimit);
        }

        // Velocity limit
        if sample.velocity.abs() > self.config.max_velocity * 1.1 {
            defmt::error!("Velocity limit exceeded: {} rad/s", sample.velocity);
            return Err(ErrorCode::VelocityLimit);
        }

        // Current limit
        if sample.current_iq.abs() > self.config.max_current * 1.1 {
            defmt::error!("Current limit exceeded: {} A", sample.current_iq);
            return Err(ErrorCode::CurrentLimit);
        }

        // Temperature limit (hard-coded 80°C)
        if sample.temperature > 80.0 {
            defmt::error!("Temperature limit exceeded: {} °C", sample.temperature);
            return Err(ErrorCode::TemperatureLimit);
        }

        // Phase timeout
        let phase_duration = sample.timestamp - self.phase_start_time;
        if phase_duration > self.config.phase_timeout {
            defmt::error!("Phase timeout: {}s > {}s", phase_duration, self.config.phase_timeout);
            return Err(ErrorCode::Timeout);
        }

        Ok(())
    }

    /// Get total elapsed time
    pub fn elapsed_time(&self, current_time: f32) -> f32 {
        current_time - self.start_time
    }

    /// Get phase elapsed time
    pub fn phase_elapsed(&self, current_time: f32) -> f32 {
        current_time - self.phase_start_time
    }
}
