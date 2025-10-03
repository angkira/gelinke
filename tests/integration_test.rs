// Integration tests for joint_firmware
// These tests verify module interactions without requiring hardware

#![cfg(test)]

use joint_firmware::firmware::config::{MotorConfig, EncoderConfig};
use joint_firmware::firmware::control::position::PositionController;
use joint_firmware::firmware::control::velocity::VelocityController;
use joint_firmware::firmware::control::observer::LuenbergerObserver;
use joint_firmware::firmware::drivers::can::{CanCommand, CanFrame};
use joint_firmware::firmware::drivers::adc::CurrentSensors;
use joint_firmware::firmware::drivers::sensors::AngleSensor;
use fixed::types::I16F16;

#[test]
fn test_cascaded_control_chain() {
    // Position → Velocity → Current
    let mut pos_ctrl = PositionController::new(Default::default());
    let mut vel_ctrl = VelocityController::new(Default::default());
    
    // Set target position
    pos_ctrl.set_target(I16F16::from_num(1.0)); // 1 radian
    
    // Current position at 0
    let vel_setpoint = pos_ctrl.update(I16F16::ZERO);
    
    // Velocity setpoint should be positive
    assert!(vel_setpoint > I16F16::ZERO);
    
    // Velocity controller with 0 current velocity
    vel_ctrl.set_target(vel_setpoint);
    let current_setpoint = vel_ctrl.update(I16F16::ZERO, 0.0001);
    
    // Current setpoint should be positive
    assert!(current_setpoint > I16F16::ZERO);
}

#[test]
fn test_observer_with_control() {
    let mut observer = LuenbergerObserver::new(Default::default());
    
    // Initialize at zero
    observer.reset(I16F16::ZERO, I16F16::ZERO);
    
    // Apply motor torque
    let motor_torque = I16F16::from_num(0.1);
    
    // Run observer for several iterations
    for _ in 0..10 {
        let _load_est = observer.update(
            I16F16::ZERO,
            I16F16::from_num(1.0),
            motor_torque,
            0.0001,
        );
    }
    
    // Velocity estimate should converge toward measurement
    let vel_est = observer.velocity_estimate();
    assert!(vel_est.to_num::<f32>() > 0.0);
}

#[test]
fn test_can_command_to_control_flow() {
    // Simulate CAN command → Control setpoint flow
    let frame = CanFrame::new(0x01).with_data(&[
        CanCommand::SetPosition as u8,
        0x00, 0x10, 0x00, 0x00, // 0x1000 = 4096 in little-endian
    ]);
    
    let cmd = frame.parse_command().unwrap();
    assert_eq!(cmd, CanCommand::SetPosition);
    
    let position_raw = frame.get_i32(1).unwrap();
    assert_eq!(position_raw, 4096);
    
    // Convert to radians (assuming milliradians)
    let position_rad = I16F16::from_num(position_raw as f32 / 1000.0);
    
    // Apply to position controller
    let mut pos_ctrl = PositionController::new(Default::default());
    pos_ctrl.set_target(position_rad);
    
    assert_eq!(pos_ctrl.target(), position_rad);
}

#[test]
fn test_motor_config() {
    let config = MotorConfig::default();
    assert_eq!(config.pole_pairs, 7);
}

#[test]
fn test_encoder_config() {
    let config = EncoderConfig::tle5012b();
    assert_eq!(config.resolution_bits, 15);
}

#[test]
fn test_adc_conversion_consistency() {
    // Test that ADC conversion is bidirectional
    let offset = 2048;
    
    // Positive current
    let raw_pos = 2548;
    let current_pos = CurrentSensors::raw_to_milliamps(raw_pos, offset);
    assert!(current_pos > 0);
    
    // Negative current
    let raw_neg = 1548;
    let current_neg = CurrentSensors::raw_to_milliamps(raw_neg, offset);
    assert!(current_neg < 0);
    
    // Symmetry check (approximate)
    assert!((current_pos + current_neg).abs() < 100); // Within 100 mA
}

#[test]
fn test_encoder_angle_wrapping() {
    let pole_pairs = 7;
    
    // Test angle wrapping at full revolution
    let angle_0 = AngleSensor::raw_to_electrical_angle_mdeg(0, pole_pairs);
    let angle_max = AngleSensor::raw_to_electrical_angle_mdeg(32767, pole_pairs);
    
    // Both should be close to 0 or 360k (due to wrapping)
    assert!(angle_0 < 360_000);
    assert!(angle_max < 360_000);
}

#[test]
fn test_full_control_pipeline() {
    // Simulate complete control pipeline
    let mut pos_ctrl = PositionController::new(Default::default());
    let mut vel_ctrl = VelocityController::new(Default::default());
    let mut observer = LuenbergerObserver::new(Default::default());
    
    // Set position target
    pos_ctrl.set_target(I16F16::from_num(2.0));
    
    // Simulate control loop
    let mut current_pos = I16F16::ZERO;
    let mut current_vel = I16F16::ZERO;
    
    for i in 0..100 {
        // Position control
        let vel_setpoint = pos_ctrl.update(current_pos);
        
        // Velocity control
        vel_ctrl.set_target(vel_setpoint);
        let current_setpoint = vel_ctrl.update(current_vel, 0.001);
        
        // Observer (using current as torque proxy)
        let motor_torque = current_setpoint * I16F16::from_num(0.1); // Kt = 0.1 Nm/A
        observer.update(current_pos, current_vel, motor_torque, 0.001);
        
        // Simulate simple plant dynamics
        current_vel += current_setpoint * I16F16::from_num(0.01);
        current_pos += current_vel * I16F16::from_num(0.001);
        
        // Check convergence after 50 iterations
        if i > 50 {
            let pos_error = (pos_ctrl.target() - current_pos).abs();
            assert!(pos_error < I16F16::from_num(0.5)); // Within 0.5 rad
        }
    }
}

