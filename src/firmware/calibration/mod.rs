//! Motor parameter calibration manager

pub mod config;
pub mod types;
pub mod safety;
pub mod tests;

use defmt::Format;
use config::{CalibrationConfig, PHASE_INERTIA, PHASE_FRICTION};
use types::{CalibrationState, ErrorCode, TestResults};
use safety::SafetyMonitor;
use tests::{InertiaTest, FrictionTest};

// Re-export for iRPC integration
pub use types::MeasurementSample;

pub struct CalibrationManager {
    state: CalibrationState,
    config: CalibrationConfig,
    safety: Option<SafetyMonitor>,

    // Test modules
    inertia_test: Option<InertiaTest>,
    friction_test: Option<FrictionTest>,

    // Results
    results: TestResults,

    // Timing
    start_time: f32,
}

impl CalibrationManager {
    pub fn new() -> Self {
        Self {
            state: CalibrationState::Idle,
            config: CalibrationConfig {
                phases: 0,
                max_current: 0.0,
                max_velocity: 0.0,
                max_position_range: 0.0,
                phase_timeout: 0.0,
                return_home: false,
                home_position: 0.0,
            },
            safety: None,
            inertia_test: None,
            friction_test: None,
            results: TestResults {
                inertia: None,
                friction: None,
                torque_constant: None,
                damping: None,
            },
            start_time: 0.0,
        }
    }

    /// Start calibration
    pub fn start(&mut self, mut config: CalibrationConfig, current_position: f32, current_time: f32) -> Result<(), ErrorCode> {
        if self.state != CalibrationState::Idle {
            return Err(ErrorCode::InvalidState);
        }

        config.validate().map_err(|_| ErrorCode::InvalidState)?;
        config.home_position = current_position;

        self.config = config;
        self.safety = Some(SafetyMonitor::new(config, current_position, current_time));
        self.start_time = current_time;
        self.state = CalibrationState::Initializing;

        defmt::info!("Calibration started at position {} rad", current_position);

        Ok(())
    }

    /// Stop/abort calibration
    pub fn stop(&mut self) {
        if self.state != CalibrationState::Idle {
            defmt::warn!("Calibration aborted by user");
            self.state = CalibrationState::Failed(ErrorCode::UserAbort);
        }
    }

    /// Update calibration (call at control frequency)
    /// Returns: (i_q_command, velocity_command, is_complete)
    pub fn update(&mut self, sample: MeasurementSample) -> (f32, f32, bool) {
        // Safety check
        if let Some(ref safety) = self.safety {
            if let Err(error) = safety.check_safety(&sample) {
                self.state = CalibrationState::Failed(error);
                return (0.0, 0.0, true);
            }
        }

        match self.state {
            CalibrationState::Initializing => {
                // Transition to first enabled phase
                if self.config.phase_enabled(PHASE_INERTIA) {
                    self.inertia_test = Some(InertiaTest::new(0.15));  // Default kt
                    self.state = CalibrationState::InertiaTest;
                    if let Some(ref mut safety) = self.safety {
                        safety.reset_phase_timer(sample.timestamp);
                    }
                } else if self.config.phase_enabled(PHASE_FRICTION) {
                    self.friction_test = Some(FrictionTest::new(0.15, self.config.max_velocity));
                    self.state = CalibrationState::FrictionTest;
                    if let Some(ref mut safety) = self.safety {
                        safety.reset_phase_timer(sample.timestamp);
                    }
                } else {
                    // No phases enabled
                    self.state = CalibrationState::Complete;
                    return (0.0, 0.0, true);
                }
                (0.0, 0.0, false)
            }

            CalibrationState::InertiaTest => {
                if let Some(ref mut test) = self.inertia_test {
                    let (i_q, complete) = test.update(sample);

                    if complete {
                        let result = test.get_result();
                        let inertia_J = result.J;
                        defmt::info!("Inertia test complete: J = {} kg·m²", inertia_J);
                        self.results.inertia = Some(result);

                        // Move to next phase
                        if self.config.phase_enabled(PHASE_FRICTION) {
                            let kt = inertia_J;  // Use measured J as kt for friction test
                            self.friction_test = Some(FrictionTest::new(kt, self.config.max_velocity));
                            self.state = CalibrationState::FrictionTest;
                            if let Some(ref mut safety) = self.safety {
                                safety.reset_phase_timer(sample.timestamp);
                            }
                        } else {
                            self.state = CalibrationState::Complete;
                            return (0.0, 0.0, true);
                        }
                    }

                    (i_q, 0.0, false)
                } else {
                    (0.0, 0.0, false)
                }
            }

            CalibrationState::FrictionTest => {
                if let Some(ref mut test) = self.friction_test {
                    let (vel, complete) = test.update(sample);

                    if complete {
                        let result = test.get_result();
                        defmt::info!("Friction test complete: τ_c = {} Nm, b_f = {} Nm·s/rad",
                                     result.tau_coulomb, result.b_viscous);
                        self.results.friction = Some(result);

                        // TODO: Add more phases
                        self.state = CalibrationState::Complete;
                        return (0.0, 0.0, true);
                    }

                    (0.0, vel, false)
                } else {
                    (0.0, 0.0, false)
                }
            }

            CalibrationState::Complete | CalibrationState::Failed(_) => {
                (0.0, 0.0, true)
            }

            _ => (0.0, 0.0, false),
        }
    }

    /// Get current status for iRPC
    pub fn get_status(&self, current_time: f32) -> CalibrationStatus {
        let (phase, progress, time_remaining) = match self.state {
            CalibrationState::InertiaTest => {
                let progress = self.inertia_test.as_ref().map(|t| t.get_progress()).unwrap_or(0.0);
                (CalibrationPhase::InertiaTest, progress, 15.0 * (1.0 - progress))
            }
            CalibrationState::FrictionTest => {
                let progress = self.friction_test.as_ref().map(|t| t.get_progress()).unwrap_or(0.0);
                (CalibrationPhase::FrictionTest, progress, 30.0 * (1.0 - progress))
            }
            CalibrationState::Complete => (CalibrationPhase::Complete, 1.0, 0.0),
            CalibrationState::Failed(_) => (CalibrationPhase::Failed, 1.0, 0.0),
            _ => (CalibrationPhase::Idle, 0.0, 0.0),
        };

        CalibrationStatus {
            phase,
            progress,
            time_remaining,
            current_position: 0.0,  // Fill from actual state
            current_velocity: 0.0,
            current_iq: 0.0,
        }
    }

    /// Get final result for iRPC
    pub fn get_result(&self, current_time: f32) -> CalibrationResult {
        let success = matches!(self.state, CalibrationState::Complete);

        let error_code = match self.state {
            CalibrationState::Failed(code) => code as u16,
            _ => 0,
        };

        // Build final parameters
        let parameters = MotorParameters {
            inertia_J: self.results.inertia.as_ref().map(|r| r.J).unwrap_or(0.001),
            torque_constant_kt: self.results.torque_constant.unwrap_or(0.15),
            damping_b: self.results.damping.unwrap_or(0.0005),
            friction_coulomb: self.results.friction.as_ref().map(|r| r.tau_coulomb).unwrap_or(0.02),
            friction_stribeck: self.results.friction.as_ref().map(|r| r.tau_stribeck).unwrap_or(0.01),
            friction_vstribeck: self.results.friction.as_ref().map(|r| r.v_stribeck).unwrap_or(0.1),
            friction_viscous: self.results.friction.as_ref().map(|r| r.b_viscous).unwrap_or(0.001),
        };

        let confidence = CalibrationConfidence {
            overall: 0.85,  // TODO: Calculate from individual confidences
            inertia: self.results.inertia.as_ref().map(|r| r.confidence).unwrap_or(0.0),
            friction: self.results.friction.as_ref().map(|r| r.confidence).unwrap_or(0.0),
            torque_constant: 0.0,
            validation_rms: 0.0,
        };

        let total_time = if let Some(ref safety) = self.safety {
            safety.elapsed_time(current_time)
        } else {
            0.0
        };

        CalibrationResult {
            success,
            parameters,
            confidence,
            total_time,
            error_code,
        }
    }

    pub fn is_complete(&self) -> bool {
        matches!(self.state, CalibrationState::Complete | CalibrationState::Failed(_))
    }

    pub fn state(&self) -> CalibrationState {
        self.state
    }
}

impl Default for CalibrationManager {
    fn default() -> Self {
        Self::new()
    }
}

// Stub types matching iRPC (will be replaced with actual iRPC imports)
#[derive(Format)]
pub struct CalibrationStatus {
    pub phase: CalibrationPhase,
    pub progress: f32,
    pub time_remaining: f32,
    pub current_position: f32,
    pub current_velocity: f32,
    pub current_iq: f32,
}

#[derive(Format, PartialEq)]
pub enum CalibrationPhase {
    Idle,
    InertiaTest,
    FrictionTest,
    TorqueConstantVerification,
    DampingTest,
    Validation,
    Complete,
    Failed,
}

pub struct CalibrationResult {
    pub success: bool,
    pub parameters: MotorParameters,
    pub confidence: CalibrationConfidence,
    pub total_time: f32,
    pub error_code: u16,
}

pub struct MotorParameters {
    pub inertia_J: f32,
    pub torque_constant_kt: f32,
    pub damping_b: f32,
    pub friction_coulomb: f32,
    pub friction_stribeck: f32,
    pub friction_vstribeck: f32,
    pub friction_viscous: f32,
}

pub struct CalibrationConfidence {
    pub overall: f32,
    pub inertia: f32,
    pub friction: f32,
    pub torque_constant: f32,
    pub validation_rms: f32,
}
