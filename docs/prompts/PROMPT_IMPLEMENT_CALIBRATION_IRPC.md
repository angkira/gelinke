# PROMPT: Implement Motor Calibration Protocol in iRPC

**Task:** Add motor parameter calibration support to iRPC protocol library

**Context:** We need automatic motor parameter identification (J, kt, b, friction) through controlled tests. The joint firmware will execute the tests, iRPC provides the protocol layer.

---

## Your Mission

Implement the calibration protocol extension in the `iRPC` repository following the specification in `/joint_firmware/docs/IRPC_V2_CALIBRATION.md`.

**Deliverables:**
1. ✅ Protocol structures in `iRPC/src/protocol.rs`
2. ✅ Python bindings if `arm_api` feature enabled
3. ✅ Example client code
4. ✅ Unit tests for serialization/deserialization
5. ✅ Update `CHANGELOG.md` for v2.1.0

---

## Step 1: Add Protocol Structures

**File:** `iRPC/src/protocol.rs`

### Add New Structs

```rust
/// Calibration request configuration
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct CalibrationRequest {
    /// Phases to run (bitmask: bit 0 = Inertia, bit 1 = Friction, bit 2 = TorqueConstant, bit 3 = Damping, bit 4 = Validation)
    pub phases: u8,
    /// Maximum test current (A)
    pub max_current: f32,
    /// Maximum test velocity (rad/s)
    pub max_velocity: f32,
    /// Maximum position excursion from start (rad)
    pub max_position_range: f32,
    /// Safety timeout per phase (seconds)
    pub phase_timeout: f32,
    /// Return to home after completion
    pub return_home: bool,
}

impl Default for CalibrationRequest {
    fn default() -> Self {
        Self {
            phases: 0b11111,  // All phases
            max_current: 8.0,
            max_velocity: 5.0,
            max_position_range: 3.14,  // ±180°
            phase_timeout: 60.0,
            return_home: true,
        }
    }
}

/// Calibration phase identifiers
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CalibrationPhase {
    Idle = 0,
    InertiaTest = 1,
    FrictionTest = 2,
    TorqueConstantVerification = 3,
    DampingTest = 4,
    Validation = 5,
    Complete = 6,
    Failed = 7,
}

/// Calibration status update (sent periodically during calibration)
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CalibrationStatus {
    /// Current calibration phase
    pub phase: CalibrationPhase,
    /// Progress within current phase (0.0 - 1.0)
    pub progress: f32,
    /// Estimated time remaining (seconds)
    pub time_remaining: f32,
    /// Current position (rad)
    pub current_position: f32,
    /// Current velocity (rad/s)
    pub current_velocity: f32,
    /// Current test current (A)
    pub current_iq: f32,
}

/// Identified motor parameters
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct MotorParameters {
    /// Rotor inertia (kg·m²)
    pub inertia_J: f32,
    /// Torque constant (Nm/A)
    pub torque_constant_kt: f32,
    /// Viscous damping (Nm·s/rad)
    pub damping_b: f32,
    /// Coulomb friction (Nm)
    pub friction_coulomb: f32,
    /// Stribeck friction amplitude (Nm)
    pub friction_stribeck: f32,
    /// Stribeck velocity (rad/s)
    pub friction_vstribeck: f32,
    /// Viscous friction coefficient (Nm·s/rad)
    pub friction_viscous: f32,
}

/// Calibration confidence metrics
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CalibrationConfidence {
    /// Overall confidence (0.0 - 1.0)
    pub overall: f32,
    /// Inertia confidence (based on measurement variance)
    pub inertia: f32,
    /// Friction model fit quality (R² score)
    pub friction: f32,
    /// Torque constant confidence
    pub torque_constant: f32,
    /// Validation tracking RMS error (rad)
    pub validation_rms: f32,
}

/// Calibration result
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CalibrationResult {
    /// Calibration success flag
    pub success: bool,
    /// Identified motor parameters
    pub parameters: MotorParameters,
    /// Confidence metrics
    pub confidence: CalibrationConfidence,
    /// Total calibration time (seconds)
    pub total_time: f32,
    /// Error code (0 = success, non-zero = error)
    pub error_code: u16,
}
```

### Update Payload Enum

```rust
pub enum Payload {
    // ... existing variants ...

    // Motor Calibration (v2.1) - Phase 6
    /// Start automatic motor parameter calibration
    StartCalibration(CalibrationRequest),
    /// Stop/abort ongoing calibration
    StopCalibration,
    /// Calibration status update (Joint → Arm, sent every 100ms during calibration)
    CalibrationStatus(CalibrationStatus),
    /// Calibration final result (Joint → Arm, sent once at end)
    CalibrationResult(CalibrationResult),
}
```

### Update LifecycleState Enum

```rust
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LifecycleState {
    /// Joint is not configured and cannot accept commands
    Unconfigured = 0,
    /// Joint is configured but not ready for motion
    Inactive = 1,
    /// Joint is active and can execute motion commands
    Active = 2,
    /// Joint is performing automatic calibration
    Calibrating = 3,  // NEW
    /// Joint is in error state
    Error = 4,
}
```

---

## Step 2: Update Tests

**File:** `iRPC/tests/protocol_tests.rs` (create if doesn't exist)

```rust
#[cfg(test)]
mod calibration_tests {
    use super::*;
    use irpc::protocol::*;

    #[test]
    fn test_calibration_request_serialization() {
        let request = CalibrationRequest {
            phases: 0b11111,
            max_current: 8.0,
            max_velocity: 5.0,
            max_position_range: 3.14,
            phase_timeout: 60.0,
            return_home: true,
        };

        let msg = Message {
            header: Header {
                source_id: 0x0000,
                target_id: 0x0010,
                msg_id: 42,
            },
            payload: Payload::StartCalibration(request),
        };

        // Serialize
        let bytes = msg.serialize().expect("Serialization failed");
        
        // Deserialize
        let decoded = Message::deserialize(&bytes).expect("Deserialization failed");

        // Verify
        match decoded.payload {
            Payload::StartCalibration(req) => {
                assert_eq!(req.phases, 0b11111);
                assert_eq!(req.max_current, 8.0);
                assert!(req.return_home);
            }
            _ => panic!("Wrong payload type"),
        }
    }

    #[test]
    fn test_calibration_status_roundtrip() {
        let status = CalibrationStatus {
            phase: CalibrationPhase::FrictionTest,
            progress: 0.65,
            time_remaining: 12.5,
            current_position: 1.2,
            current_velocity: 2.5,
            current_iq: 3.0,
        };

        let msg = Message {
            header: Header {
                source_id: 0x0010,
                target_id: 0x0000,
                msg_id: 100,
            },
            payload: Payload::CalibrationStatus(status),
        };

        let bytes = msg.serialize().unwrap();
        let decoded = Message::deserialize(&bytes).unwrap();

        match decoded.payload {
            Payload::CalibrationStatus(s) => {
                assert_eq!(s.phase, CalibrationPhase::FrictionTest);
                assert!((s.progress - 0.65).abs() < 0.01);
            }
            _ => panic!("Wrong payload"),
        }
    }

    #[test]
    fn test_calibration_result_complete() {
        let result = CalibrationResult {
            success: true,
            parameters: MotorParameters {
                inertia_J: 0.001,
                torque_constant_kt: 0.15,
                damping_b: 0.0005,
                friction_coulomb: 0.02,
                friction_stribeck: 0.01,
                friction_vstribeck: 0.1,
                friction_viscous: 0.001,
            },
            confidence: CalibrationConfidence {
                overall: 0.92,
                inertia: 0.95,
                friction: 0.88,
                torque_constant: 0.94,
                validation_rms: 0.015,
            },
            total_time: 62.5,
            error_code: 0,
        };

        let msg = Message {
            header: Header {
                source_id: 0x0010,
                target_id: 0x0000,
                msg_id: 200,
            },
            payload: Payload::CalibrationResult(result),
        };

        let bytes = msg.serialize().unwrap();
        assert!(bytes.len() < Message::max_size());
        
        let decoded = Message::deserialize(&bytes).unwrap();
        match decoded.payload {
            Payload::CalibrationResult(r) => {
                assert!(r.success);
                assert!((r.parameters.inertia_J - 0.001).abs() < 1e-6);
                assert!((r.confidence.overall - 0.92).abs() < 0.01);
            }
            _ => panic!("Wrong payload"),
        }
    }

    #[test]
    fn test_default_calibration_request() {
        let default = CalibrationRequest::default();
        assert_eq!(default.phases, 0b11111);
        assert_eq!(default.max_current, 8.0);
        assert_eq!(default.max_velocity, 5.0);
        assert!(default.return_home);
    }

    #[test]
    fn test_calibration_phase_values() {
        assert_eq!(CalibrationPhase::Idle as u8, 0);
        assert_eq!(CalibrationPhase::InertiaTest as u8, 1);
        assert_eq!(CalibrationPhase::Complete as u8, 6);
        assert_eq!(CalibrationPhase::Failed as u8, 7);
    }

    #[test]
    fn test_lifecycle_state_calibrating() {
        let state = LifecycleState::Calibrating;
        assert_eq!(state as u8, 3);
    }
}
```

---

## Step 3: Example Client Code

**File:** `iRPC/examples/calibration_example.rs`

```rust
//! Example: Motor Parameter Calibration
//!
//! This example demonstrates how to:
//! 1. Start motor calibration
//! 2. Monitor calibration progress
//! 3. Receive and display results
//!
//! Usage: cargo run --example calibration_example --features arm_api

use irpc::protocol::*;
use std::time::Duration;
use std::thread;

fn main() {
    println!("🔧 Motor Calibration Example\n");

    // Create calibration request
    let request = CalibrationRequest {
        phases: 0b11111,  // All phases
        max_current: 8.0,
        max_velocity: 5.0,
        max_position_range: 3.14,
        phase_timeout: 60.0,
        return_home: true,
    };

    println!("📋 Calibration Configuration:");
    println!("  Phases: 0b{:05b} (all enabled)", request.phases);
    println!("  Max current: {:.1} A", request.max_current);
    println!("  Max velocity: {:.1} rad/s", request.max_velocity);
    println!("  Position range: ±{:.1}°", request.max_position_range * 180.0 / 3.14159);
    println!("  Phase timeout: {:.0}s", request.phase_timeout);
    println!();

    // Create message
    let msg = Message {
        header: Header {
            source_id: 0x0000,  // Arm
            target_id: 0x0010,  // Joint
            msg_id: 1,
        },
        payload: Payload::StartCalibration(request),
    };

    // Serialize
    let bytes = msg.serialize().expect("Failed to serialize");
    println!("✅ Message serialized: {} bytes", bytes.len());
    println!("   CAN frames needed: {}", (bytes.len() + 7) / 8);
    println!();

    // Simulate status updates
    println!("📊 Simulating calibration progress:\n");
    
    let phases = vec![
        (CalibrationPhase::InertiaTest, "Inertia Test", 15.0),
        (CalibrationPhase::FrictionTest, "Friction Test", 25.0),
        (CalibrationPhase::TorqueConstantVerification, "Torque Constant Verification", 8.0),
        (CalibrationPhase::DampingTest, "Damping Test", 12.0),
        (CalibrationPhase::Validation, "Validation", 5.0),
    ];

    for (phase, name, duration) in phases {
        println!("Phase: {}", name);
        let steps = 20;
        for i in 0..=steps {
            let progress = i as f32 / steps as f32;
            let time_remaining = duration * (1.0 - progress);
            
            print!("\r  [");
            for j in 0..steps {
                if j < i {
                    print!("█");
                } else {
                    print!("░");
                }
            }
            print!("] {:.0}% (ETA: {:.1}s)  ", progress * 100.0, time_remaining);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
            
            thread::sleep(Duration::from_millis((duration * 1000.0 / steps as f32) as u64 / 10));
        }
        println!();
    }

    println!("\n✅ Calibration complete!\n");

    // Simulate result
    let result = CalibrationResult {
        success: true,
        parameters: MotorParameters {
            inertia_J: 0.001052,
            torque_constant_kt: 0.1487,
            damping_b: 0.000521,
            friction_coulomb: 0.0198,
            friction_stribeck: 0.0087,
            friction_vstribeck: 0.0953,
            friction_viscous: 0.001034,
        },
        confidence: CalibrationConfidence {
            overall: 0.91,
            inertia: 0.94,
            friction: 0.87,
            torque_constant: 0.93,
            validation_rms: 0.0172,
        },
        total_time: 63.2,
        error_code: 0,
    };

    println!("📊 Motor Parameters:");
    println!("  J  = {:.6} kg·m²", result.parameters.inertia_J);
    println!("  kt = {:.4} Nm/A", result.parameters.torque_constant_kt);
    println!("  b  = {:.6} Nm·s/rad", result.parameters.damping_b);
    println!();
    println!("📊 Friction Model:");
    println!("  τ_coulomb  = {:.4} Nm", result.parameters.friction_coulomb);
    println!("  τ_stribeck = {:.4} Nm", result.parameters.friction_stribeck);
    println!("  v_stribeck = {:.4} rad/s", result.parameters.friction_vstribeck);
    println!("  b_viscous  = {:.6} Nm·s/rad", result.parameters.friction_viscous);
    println!();
    println!("🎯 Confidence Metrics:");
    println!("  Overall:       {:.1}%", result.confidence.overall * 100.0);
    println!("  Inertia:       {:.1}%", result.confidence.inertia * 100.0);
    println!("  Friction:      {:.1}%", result.confidence.friction * 100.0);
    println!("  Torque const:  {:.1}%", result.confidence.torque_constant * 100.0);
    println!("  Validation RMS: {:.4} rad ({:.2}°)", 
             result.confidence.validation_rms,
             result.confidence.validation_rms * 180.0 / 3.14159);
    println!();
    println!("⏱️  Total time: {:.1}s", result.total_time);
}
```

---

## Step 4: Update Documentation

**File:** `iRPC/CHANGELOG.md`

```markdown
## [2.1.0] - 2025-10-10

### Added
- **Motor Parameter Calibration** (Phase 6)
  - `StartCalibration` command with configurable test parameters
  - `StopCalibration` command for emergency abort
  - `CalibrationStatus` telemetry (10 Hz during calibration)
  - `CalibrationResult` with identified parameters and confidence metrics
  - New `Calibrating` lifecycle state
  - Automatic identification of:
    - Rotor inertia (J)
    - Torque constant (kt)
    - Viscous damping (b)
    - Stribeck friction model (τ_c, τ_s, v_s, b_f)
  - Safety monitoring (position, velocity, current, temperature limits)
  - Confidence scoring for parameter quality
  - 5 calibration phases: Inertia, Friction, TorqueConstant, Damping, Validation

### Changed
- `LifecycleState` enum now includes `Calibrating = 3`
- `Payload` enum extended with 4 new calibration variants

### Documentation
- Added `docs/IRPC_V2_CALIBRATION.md` specification
- Added `examples/calibration_example.rs`
- Updated state transition diagram in README

### Breaking Changes
- `LifecycleState` enum values shifted (Error: 3 → 4)
- Existing code using numeric state values must be updated
```

**File:** `iRPC/README.md`

Add section:

```markdown
## Motor Calibration (v2.1)

Automatic motor parameter identification through controlled tests:

```rust
use irpc::protocol::*;

// Configure calibration
let request = CalibrationRequest {
    phases: 0b11111,  // All phases
    max_current: 8.0,
    max_velocity: 5.0,
    max_position_range: 3.14,
    phase_timeout: 60.0,
    return_home: true,
};

// Send command
let msg = Message {
    header: Header {
        source_id: 0x0000,
        target_id: 0x0010,
        msg_id: 1,
    },
    payload: Payload::StartCalibration(request),
};

// Monitor progress
// Joint will send CalibrationStatus every 100ms
// Joint will send CalibrationResult at completion
```

See [`docs/IRPC_V2_CALIBRATION.md`](docs/IRPC_V2_CALIBRATION.md) for details.
```

---

## Step 5: Version Update

**File:** `iRPC/Cargo.toml`

```toml
[package]
name = "irpc"
version = "2.1.0"  # ← Update from 2.0.x
```

---

## Validation Checklist

Before submitting, verify:

- [ ] All structs derive `Serialize`, `Deserialize`, `Debug`, `Clone`, `Copy`
- [ ] `CalibrationRequest` has `Default` implementation
- [ ] `CalibrationPhase` uses `#[repr(u8)]`
- [ ] All tests pass: `cargo test`
- [ ] Example compiles: `cargo run --example calibration_example --features arm_api`
- [ ] No breaking changes to existing protocol (except LifecycleState values)
- [ ] Serialized message size < 128 bytes (check with test)
- [ ] Documentation updated (CHANGELOG, README)
- [ ] Code follows existing style (rustfmt)

---

## Expected Timeline

- Protocol structures: **1-2 hours**
- Tests: **1 hour**
- Example: **30 minutes**
- Documentation: **30 minutes**
- Testing & polish: **1 hour**

**Total:** ~4-5 hours

---

## Questions?

If anything is unclear:
1. Read `/joint_firmware/docs/IRPC_V2_CALIBRATION.md` for full specification
2. Check existing protocol implementation in `iRPC/src/protocol.rs`
3. Look at `SetTargetPayloadV2` as reference for complex structs

---

## Success Criteria

✅ All tests pass  
✅ Example runs without errors  
✅ Message size < 128 bytes  
✅ No compiler warnings  
✅ Documentation complete  
✅ Version bumped to 2.1.0  

---

**Good luck! This is a critical feature for production deployment. 🚀**

