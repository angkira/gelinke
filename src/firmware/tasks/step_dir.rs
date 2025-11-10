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

    /// Update PWM outputs based on current position for DRV8844 H-bridge.
    ///
    /// DRV8844 requires 4 independent PWM signals for 2-phase stepper control:
    /// - AIN1/AIN2 control Phase A H-bridge
    /// - BIN1/BIN2 control Phase B H-bridge
    pub fn update_pwm(&self, pwm: &mut PhasePwm) {
        if self.state != StepDirState::Running {
            pwm.disable();
            return;
        }

        let (phase_a, phase_b) = self.table.get_phases(self.current_microstep);

        let max_duty = pwm.max_duty();

        // Convert normalized phases [-1.0, 1.0] to H-bridge control signals
        // For DRV8844:
        // - Positive phase: AIN1=PWM, AIN2=0 (forward current)
        // - Negative phase: AIN1=0, AIN2=PWM (reverse current)

        // Phase A H-bridge control
        let (a1_duty, a2_duty) = if phase_a >= 0.0 {
            // Forward: AIN1=duty, AIN2=0
            ((phase_a * max_duty as f32) as u16, 0)
        } else {
            // Reverse: AIN1=0, AIN2=duty
            (0, ((-phase_a) * max_duty as f32) as u16)
        };

        // Phase B H-bridge control
        let (b1_duty, b2_duty) = if phase_b >= 0.0 {
            // Forward: BIN1=duty, BIN2=0
            ((phase_b * max_duty as f32) as u16, 0)
        } else {
            // Reverse: BIN1=0, BIN2=duty
            (0, ((-phase_b) * max_duty as f32) as u16)
        };

        // Set all 4 H-bridge inputs: [AIN1, AIN2, BIN1, BIN2]
        pwm.set_all_duties([a1_duty, a2_duty, b1_duty, b2_duty]);
    }

    /// Get current state
    pub fn state(&self) -> StepDirState {
        self.state
    }
}

/// Main Step-Dir control loop task
///
/// NOTE: This task should be spawned from system.rs with actual hardware peripherals.
/// The GPIO interface needs to be initialized and passed in.
///
/// Example usage from system.rs:
/// ```ignore
/// use crate::firmware::drivers::step_dir_interface::StepDirInterface;
/// use crate::firmware::drivers::pwm::MotorPwm;
/// use crate::firmware::drivers::motor_driver::MotorDriver;
///
/// let step_dir_gpio = StepDirInterface::new(p);  // Takes PB5, PB4, PA8, PB3
/// let pwm = MotorPwm::new(p, DEFAULT_PWM_FREQ);   // Takes PA0, PA1, PB10, PB11
/// let motor_driver = MotorDriver::new(p);         // Takes PA4, PB1, PB2
///
/// spawner.spawn(step_dir_control_loop_with_hw(step_dir_gpio, pwm, motor_driver)).ok();
/// ```
#[embassy_executor::task]
pub async fn control_loop() {
    defmt::info!("Step-Dir control loop starting (STUB MODE - no hardware)");
    defmt::warn!("This task needs hardware peripherals to be functional");
    defmt::warn!("Use control_loop_with_hardware() from system.rs instead");

    let mut ticker = Ticker::every(Duration::from_micros(STEP_DIR_LOOP_PERIOD_US));
    let mut iteration = 0u32;

    loop {
        ticker.next().await;

        iteration = iteration.wrapping_add(1);

        // Log less frequently to reduce overhead
        if iteration % 1_000 == 0 {
            defmt::info!("Step-Dir stub loop: {} iterations (waiting for hardware integration)", iteration);
        }

        // This stub is kept for compatibility with existing test infrastructure.
        // Actual hardware control requires GPIO/PWM peripherals passed from system.rs
    }
}

/// Step-Dir control loop with hardware integration (event-driven).
///
/// This is the production implementation that uses EXTI interrupts for step pulses.
/// Much more efficient than polling - CPU only wakes up on step edges.
///
/// NOTE: This is a template showing how to integrate the hardware.
/// Uncomment and use this in system.rs when ready for hardware testing.
/*
#[embassy_executor::task]
pub async fn control_loop_with_hardware(
    mut gpio: crate::firmware::drivers::step_dir_interface::StepDirInterface,
    mut pwm: crate::firmware::drivers::pwm::MotorPwm,
    mut motor: crate::firmware::drivers::motor_driver::MotorDriver,
) {
    defmt::info!("Step-Dir control loop with hardware starting");

    // Create controller
    let config = crate::firmware::config::MotorConfig::default();
    let mut controller = StepDirController::new(&config);

    // Enable motor driver
    motor.enable();
    defmt::info!("Motor driver enabled");

    let mut step_count = 0u32;

    // Event-driven loop - waits for step pulses via EXTI interrupt
    loop {
        // Wait for step pulse (async - CPU sleeps until interrupt)
        gpio.wait_for_step().await;
        step_count = step_count.wrapping_add(1);

        // Read direction from GPIO
        let direction = gpio.read_direction();
        controller.set_direction(direction);

        // Check if enabled
        if gpio.is_enabled() {
            if controller.state() != StepDirState::Running {
                controller.enable();
                gpio.clear_error();
            }

            // Process step
            controller.step();

            // Update PWM outputs
            controller.update_pwm(&mut pwm);

            // Log periodically
            if step_count % 1000 == 0 {
                defmt::info!("Steps: {}, Position: {}", step_count, controller.position_steps());
            }
        } else {
            // Disabled via ENABLE pin
            if controller.state() != StepDirState::Idle {
                controller.disable();
                pwm.disable();
                defmt::info!("Step-Dir disabled via ENABLE pin");
            }
        }

        // Check for motor driver faults
        if motor.is_fault() {
            defmt::error!("Motor driver fault detected!");
            controller.disable();
            pwm.disable();
            gpio.set_error();
            motor.disable();
            // Could implement fault recovery here
        }
    }
}
*/

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
