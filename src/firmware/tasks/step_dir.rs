/// Classic Step-Dir stepper motor control
///
/// This module implements traditional step/direction control for stepper motors,
/// compatible with legacy step/dir interfaces. It supports microstepping and
/// provides position tracking.

use embassy_time::{Duration, Ticker};
use fixed::types::I16F16;

use crate::firmware::config::MotorConfig;
use crate::firmware::drivers::pwm::PhasePwm;

/// Step-Dir control loop frequency in Hz.
/// We run at a lower frequency than FOC since we don't need closed-loop control.
pub const STEP_DIR_LOOP_FREQ_HZ: u32 = 1_000;

/// Step-Dir control loop period in microseconds.
pub const STEP_DIR_LOOP_PERIOD_US: u64 = 1_000_000 / STEP_DIR_LOOP_FREQ_HZ as u64;

/// Maximum step frequency (steps per second)
pub const MAX_STEP_FREQ_HZ: u32 = 50_000;

/// Step-Dir controller state.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum StepDirState {
    Idle,
    Running,
    Fault,
}

/// Microstepping lookup table for sine wave approximation
/// This generates smooth microsteps between full steps
struct MicrostepTable {
    microsteps: u16,
}

impl MicrostepTable {
    /// Create a new microstepping lookup table
    pub fn new(microsteps: u16) -> Self {
        assert!(
            microsteps.is_power_of_two() && microsteps >= 1 && microsteps <= 256,
            "Microsteps must be a power of 2 between 1 and 256"
        );
        Self { microsteps }
    }

    /// Get phase currents for a given microstep position
    /// Returns (phase_a, phase_b) as normalized values [-1.0, 1.0]
    pub fn get_phases(&self, microstep: u32) -> (f32, f32) {
        let steps_per_rev = self.microsteps as u32 * 4; // 4 full steps per electrical cycle
        let angle = (microstep % steps_per_rev) as f32 * 2.0 * core::f32::consts::PI / steps_per_rev as f32;

        // Two-phase stepper: A and B phases are 90 degrees apart
        let phase_a = libm::cosf(angle);
        let phase_b = libm::sinf(angle);

        (phase_a, phase_b)
    }
}

/// Step-Dir controller
pub struct StepDirController {
    state: StepDirState,
    /// Current microstep position
    current_microstep: u32,
    /// Direction: true = forward, false = reverse
    direction: bool,
    /// Microstepping table
    table: MicrostepTable,
    /// Steps per full revolution (microsteps * 200 for a standard stepper)
    steps_per_rev: u32,
}

impl StepDirController {
    /// Create a new Step-Dir controller
    pub fn new(motor_config: &MotorConfig) -> Self {
        // Standard stepper has 200 full steps per revolution (1.8° per step)
        let full_steps_per_rev = 200;
        let steps_per_rev = full_steps_per_rev * motor_config.microsteps as u32;

        Self {
            state: StepDirState::Idle,
            current_microstep: 0,
            direction: true,
            table: MicrostepTable::new(motor_config.microsteps),
            steps_per_rev,
        }
    }

    /// Enable the controller
    pub fn enable(&mut self) {
        self.state = StepDirState::Running;
    }

    /// Disable the controller
    pub fn disable(&mut self) {
        self.state = StepDirState::Idle;
    }

    /// Set direction
    pub fn set_direction(&mut self, forward: bool) {
        self.direction = forward;
    }

    /// Process a step pulse
    pub fn step(&mut self) {
        if self.state != StepDirState::Running {
            return;
        }

        if self.direction {
            self.current_microstep = self.current_microstep.wrapping_add(1);
        } else {
            self.current_microstep = self.current_microstep.wrapping_sub(1);
        }
    }

    /// Get current position in radians
    pub fn position(&self) -> I16F16 {
        let angle_rad = (self.current_microstep as f32 * 2.0 * core::f32::consts::PI) / self.steps_per_rev as f32;
        I16F16::from_num(angle_rad)
    }

    /// Get current position in steps
    pub fn position_steps(&self) -> i32 {
        self.current_microstep as i32
    }

    /// Update PWM outputs based on current position
    pub fn update_pwm(&self, pwm: &mut PhasePwm) {
        if self.state != StepDirState::Running {
            pwm.disable();
            return;
        }

        let (phase_a, phase_b) = self.table.get_phases(self.current_microstep);

        // Convert to three-phase outputs (assuming 2-phase to 3-phase conversion)
        // For a three-phase driver, we use standard Clarke transformation
        let duty_a = ((phase_a + 1.0) / 2.0).clamp(0.0, 1.0);
        let duty_b = ((phase_b + 1.0) / 2.0).clamp(0.0, 1.0);
        let duty_c = (((-phase_a - phase_b) / 2.0 + 1.0) / 2.0).clamp(0.0, 1.0);

        let max_duty = pwm.max_duty();
        let duties = [
            (duty_a * max_duty as f32) as u16,
            (duty_b * max_duty as f32) as u16,
            (duty_c * max_duty as f32) as u16,
        ];

        pwm.set_phase_duties(duties);
    }

    /// Get current state
    pub fn state(&self) -> StepDirState {
        self.state
    }
}

/// Main Step-Dir control loop task
#[embassy_executor::task]
pub async fn control_loop() {
    defmt::info!("Step-Dir control loop starting");

    let mut ticker = Ticker::every(Duration::from_micros(STEP_DIR_LOOP_PERIOD_US));
    let mut iteration = 0u32;

    loop {
        ticker.next().await;

        iteration = iteration.wrapping_add(1);

        // Log less frequently than FOC to reduce overhead
        if iteration % 1_000 == 0 {
            defmt::info!("Step-Dir loop: {} iterations", iteration);
        }

        // TODO: Read step/dir inputs from GPIO
        // TODO: Update PWM outputs based on position
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_loop_timing() {
        assert_eq!(STEP_DIR_LOOP_FREQ_HZ, 1_000);
        assert_eq!(STEP_DIR_LOOP_PERIOD_US, 1_000);
    }

    #[test]
    fn test_microstep_table_creation() {
        // Valid power-of-2 values
        MicrostepTable::new(1);
        MicrostepTable::new(2);
        MicrostepTable::new(16);
        MicrostepTable::new(256);
    }

    #[test]
    #[should_panic]
    fn test_microstep_table_invalid() {
        // Should panic for non-power-of-2
        MicrostepTable::new(3);
    }

    #[test]
    fn test_microstep_phases() {
        let table = MicrostepTable::new(16);

        // At step 0, phase A should be maximum, phase B should be 0
        let (a, b) = table.get_phases(0);
        assert!((a - 1.0).abs() < 0.01);
        assert!(b.abs() < 0.01);

        // At 1/4 rotation, phases should be equal
        let (a, b) = table.get_phases(16);
        assert!((a.abs() - 0.707).abs() < 0.1);
        assert!((b.abs() - 0.707).abs() < 0.1);
    }

    #[test]
    fn test_controller_creation() {
        let config = MotorConfig {
            pole_pairs: 7,
            control_method: crate::firmware::config::ControlMethod::StepDir,
            microsteps: 16,
        };

        let controller = StepDirController::new(&config);
        assert_eq!(controller.state(), StepDirState::Idle);
        assert_eq!(controller.position_steps(), 0);
    }

    #[test]
    fn test_stepping() {
        let config = MotorConfig {
            pole_pairs: 7,
            control_method: crate::firmware::config::ControlMethod::StepDir,
            microsteps: 16,
        };

        let mut controller = StepDirController::new(&config);
        controller.enable();

        // Step forward
        controller.set_direction(true);
        controller.step();
        assert_eq!(controller.position_steps(), 1);

        controller.step();
        assert_eq!(controller.position_steps(), 2);

        // Step reverse
        controller.set_direction(false);
        controller.step();
        assert_eq!(controller.position_steps(), 1);
    }

    #[test]
    fn test_position_calculation() {
        let config = MotorConfig {
            pole_pairs: 7,
            control_method: crate::firmware::config::ControlMethod::StepDir,
            microsteps: 16,
        };

        let mut controller = StepDirController::new(&config);
        controller.enable();

        // One full revolution should be 200 * 16 = 3200 steps
        for _ in 0..3200 {
            controller.step();
        }

        let pos = controller.position().to_num::<f32>();
        // Should be approximately 2*PI radians
        assert!((pos - 2.0 * core::f32::consts::PI).abs() < 0.01);
    }
}
