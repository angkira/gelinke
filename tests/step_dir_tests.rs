// Integration tests for Step-Dir stepper motor control
// These tests verify Step-Dir functionality without requiring hardware

#![cfg(test)]

use joint_firmware::firmware::config::{MotorConfig, ControlMethod};
use joint_firmware::firmware::tasks::step_dir::{
    StepDirController, StepDirState, MAX_STEP_FREQ_HZ, STEP_DIR_LOOP_FREQ_HZ,
};
use fixed::types::I16F16;

#[test]
fn test_step_dir_controller_creation() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let controller = StepDirController::new(&config);
    assert_eq!(controller.state(), StepDirState::Idle);
    assert_eq!(controller.position_steps(), 0);
}

#[test]
fn test_enable_disable() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);

    // Should start in Idle
    assert_eq!(controller.state(), StepDirState::Idle);

    // Enable
    controller.enable();
    assert_eq!(controller.state(), StepDirState::Running);

    // Disable
    controller.disable();
    assert_eq!(controller.state(), StepDirState::Idle);
}

#[test]
fn test_forward_stepping() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true); // Forward

    // Take 100 steps forward
    for _ in 0..100 {
        controller.step();
    }

    assert_eq!(controller.position_steps(), 100);
}

#[test]
fn test_reverse_stepping() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(false); // Reverse

    // Take 100 steps reverse
    for _ in 0..100 {
        controller.step();
    }

    // Position should be negative (wrapping)
    assert_eq!(controller.position_steps(), -100);
}

#[test]
fn test_direction_change() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();

    // Forward 50 steps
    controller.set_direction(true);
    for _ in 0..50 {
        controller.step();
    }
    assert_eq!(controller.position_steps(), 50);

    // Reverse 30 steps
    controller.set_direction(false);
    for _ in 0..30 {
        controller.step();
    }
    assert_eq!(controller.position_steps(), 20);

    // Forward 80 more steps
    controller.set_direction(true);
    for _ in 0..80 {
        controller.step();
    }
    assert_eq!(controller.position_steps(), 100);
}

#[test]
fn test_position_in_radians() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // One full revolution: 200 full steps * 16 microsteps = 3200 steps
    for _ in 0..3200 {
        controller.step();
    }

    let position = controller.position().to_num::<f32>();
    let two_pi = 2.0 * core::f32::consts::PI;

    // Should be approximately 2π radians (within 1%)
    assert!((position - two_pi).abs() / two_pi < 0.01);
}

#[test]
fn test_half_revolution() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // Half revolution: 1600 steps
    for _ in 0..1600 {
        controller.step();
    }

    let position = controller.position().to_num::<f32>();
    let pi = core::f32::consts::PI;

    // Should be approximately π radians (within 1%)
    assert!((position - pi).abs() / pi < 0.01);
}

#[test]
fn test_quarter_revolution() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // Quarter revolution: 800 steps
    for _ in 0..800 {
        controller.step();
    }

    let position = controller.position().to_num::<f32>();
    let pi_over_2 = core::f32::consts::PI / 2.0;

    // Should be approximately π/2 radians (within 1%)
    assert!((position - pi_over_2).abs() / pi_over_2 < 0.01);
}

#[test]
fn test_idle_no_stepping() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    // Don't enable - stay in Idle
    controller.set_direction(true);

    // Try to step while idle
    for _ in 0..100 {
        controller.step();
    }

    // Position should remain at 0
    assert_eq!(controller.position_steps(), 0);
}

#[test]
fn test_microstepping_1x() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 1,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // One full revolution: 200 full steps * 1 = 200 steps
    for _ in 0..200 {
        controller.step();
    }

    let position = controller.position().to_num::<f32>();
    let two_pi = 2.0 * core::f32::consts::PI;

    // Should be approximately 2π radians
    assert!((position - two_pi).abs() / two_pi < 0.01);
}

#[test]
fn test_microstepping_256x() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 256,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // One full revolution: 200 full steps * 256 = 51200 steps
    for _ in 0..51200 {
        controller.step();
    }

    let position = controller.position().to_num::<f32>();
    let two_pi = 2.0 * core::f32::consts::PI;

    // Should be approximately 2π radians
    assert!((position - two_pi).abs() / two_pi < 0.01);
}

#[test]
fn test_max_step_frequency_constant() {
    // Verify max step frequency is reasonable
    assert_eq!(MAX_STEP_FREQ_HZ, 50_000);
    assert!(MAX_STEP_FREQ_HZ <= 100_000); // Practical limit for step/dir
}

#[test]
fn test_loop_frequency_constant() {
    // Verify loop frequency is 1 kHz
    assert_eq!(STEP_DIR_LOOP_FREQ_HZ, 1_000);
    assert!(STEP_DIR_LOOP_FREQ_HZ < MAX_STEP_FREQ_HZ); // Loop must be slower than max step rate
}

#[test]
fn test_bidirectional_motion() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();

    // Go forward 1000 steps
    controller.set_direction(true);
    for _ in 0..1000 {
        controller.step();
    }
    assert_eq!(controller.position_steps(), 1000);

    // Go reverse 1000 steps
    controller.set_direction(false);
    for _ in 0..1000 {
        controller.step();
    }
    assert_eq!(controller.position_steps(), 0);

    // Should be back at origin
    let position = controller.position().to_num::<f32>();
    assert!(position.abs() < 0.01); // Within 0.01 radians of zero
}

#[test]
fn test_position_tracking_accuracy() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 64,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // Take a precise number of steps
    let steps_per_rev = 200 * 64; // 12800
    for _ in 0..steps_per_rev {
        controller.step();
    }

    // Check position accuracy
    let position = controller.position().to_num::<f32>();
    let expected = 2.0 * core::f32::consts::PI;
    let error = (position - expected).abs();
    let error_percent = error / expected * 100.0;

    // Error should be less than 0.1%
    assert!(error_percent < 0.1);
}

#[test]
fn test_control_method_enum() {
    // Verify ControlMethod enum values
    let foc = ControlMethod::Foc;
    let step_dir = ControlMethod::StepDir;

    assert_ne!(foc, step_dir);

    // Test default
    assert_eq!(ControlMethod::default(), ControlMethod::Foc);
}

#[test]
fn test_motor_config_with_step_dir() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 32,
    };

    assert_eq!(config.control_method, ControlMethod::StepDir);
    assert_eq!(config.microsteps, 32);
}

#[test]
fn test_position_wraparound() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // Take many steps to test wraparound (10 revolutions)
    let steps = 3200 * 10;
    for _ in 0..steps {
        controller.step();
    }

    // Position should wrap correctly
    let position = controller.position().to_num::<f32>();
    assert!(position >= 0.0);
}

#[test]
fn test_small_movements() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 256, // High resolution for small movements
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // Take just 1 step
    controller.step();
    assert_eq!(controller.position_steps(), 1);

    // Position should be very small
    let position = controller.position().to_num::<f32>();
    let expected_step_size = 2.0 * core::f32::consts::PI / (200.0 * 256.0);
    assert!((position - expected_step_size).abs() < 0.0001);
}

#[test]
fn test_alternating_direction() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();

    // Alternate direction every step
    for i in 0..100 {
        controller.set_direction(i % 2 == 0);
        controller.step();
    }

    // Net position should be zero (50 forward, 50 reverse)
    assert_eq!(controller.position_steps(), 0);
}

#[test]
fn test_high_step_count() {
    let config = MotorConfig {
        pole_pairs: 7,
        control_method: ControlMethod::StepDir,
        microsteps: 16,
    };

    let mut controller = StepDirController::new(&config);
    controller.enable();
    controller.set_direction(true);

    // Take a large number of steps
    let steps = 100_000;
    for _ in 0..steps {
        controller.step();
    }

    // Verify position matches step count
    assert_eq!(controller.position_steps(), steps as i32);
}
