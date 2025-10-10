# iRPC v2.0 - Motor Parameter Calibration

**Version:** 2.0.1  
**Date:** 2025-10-10  
**Status:** Design Specification  
**Feature:** Automatic Motor Parameter Identification

---

## Overview

Automatic motor parameter calibration allows the joint to identify its physical characteristics through a series of controlled tests. This eliminates manual measurement and provides accurate parameters for optimal control performance.

### Calibrated Parameters

| Parameter | Symbol | Unit | Description |
|-----------|--------|------|-------------|
| **Inertia** | J | kg·m² | Rotor + load inertia |
| **Torque Constant** | kt | Nm/A | Motor torque per ampere |
| **Damping** | b | Nm·s/rad | Viscous damping coefficient |
| **Coulomb Friction** | τ_c | Nm | Static friction torque |
| **Stribeck Friction** | τ_s | Nm | Stribeck friction amplitude |
| **Stribeck Velocity** | v_s | rad/s | Stribeck transition velocity |
| **Viscous Friction** | b_f | Nm·s/rad | Velocity-dependent friction |

---

## Calibration Protocol

### Workflow

```
1. StartCalibration(config) → Ack
   ├─ Joint validates state (must be Inactive or Active)
   ├─ Joint stores current position as "home"
   └─ Joint transitions to Calibrating state

2. CalibrationStatus (periodic updates)
   ├─ Current phase (InertiaTest, FrictionTest, etc.)
   ├─ Progress percentage
   └─ Estimated time remaining

3. CalibrationResult → Complete
   ├─ All identified parameters
   ├─ Confidence metrics
   └─ Joint returns to home position
   └─ Joint returns to previous state
```

### Calibration Phases

#### Phase 1: Inertia Identification (15-20s)
**Test:** Step torque input, measure acceleration response  
**Method:** τ = J·α → J = τ / α  
**Steps:**
1. Apply constant current i_q = 2A for 1s
2. Measure angular acceleration α
3. Calculate: J = (kt · i_q) / α
4. Repeat 5 times with different currents (1A, 2A, 3A, 4A, 5A)
5. Average results for robustness

**Safety:** Monitor position limits, stop if position > 180°

#### Phase 2: Friction Identification (20-30s)
**Test:** Constant velocity tracking at multiple speeds  
**Method:** At steady state, τ_friction = kt · i_q  
**Steps:**
1. Ramp up to v = 0.5 rad/s, hold 3s, record i_q
2. Ramp to v = 1.0 rad/s, hold 3s, record i_q
3. Ramp to v = 2.0 rad/s, hold 3s, record i_q
4. Ramp to v = 4.0 rad/s, hold 3s, record i_q
5. Repeat in reverse direction
6. Fit Stribeck model: τ_f(v) = τ_c·sgn(v) + τ_s·exp(-(v/v_s)²) + b_f·v

**Safety:** Monitor velocity limits, current limits

#### Phase 3: Torque Constant Verification (5-10s)
**Test:** Static torque holding against known friction  
**Method:** kt = τ / i_q (using friction model from Phase 2)  
**Steps:**
1. Move to vertical position (gravity torque known if configured)
2. Hold position with minimal motion
3. Measure steady-state i_q
4. Calculate: kt = τ_gravity / i_q

**Optional:** Requires gravity vector configuration

#### Phase 4: Damping Identification (10-15s)
**Test:** Free oscillation decay (if flexible coupling present)  
**Method:** Measure oscillation decay rate  
**Steps:**
1. Apply impulse torque (0.5 Nm for 0.1s)
2. Release and observe free oscillation
3. Measure decay rate: b = 2·ζ·√(J·k_spring)
4. If no oscillation detected, b = 0 (rigid coupling)

**Safety:** Monitor oscillation amplitude

#### Phase 5: Validation (5s)
**Test:** Execute test trajectory with identified parameters  
**Steps:**
1. Move to target position using new parameters
2. Measure tracking error
3. Report confidence score based on error

---

## iRPC Protocol Extensions

### New Payload Types

Add to `iRPC/src/protocol.rs`:

```rust
/// Calibration request configuration
#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct CalibrationRequest {
    /// Phases to run (bitmask: bit 0 = Inertia, bit 1 = Friction, etc.)
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

/// Add to Payload enum:
pub enum Payload {
    // ... existing variants ...

    // Motor Calibration (v2.1)
    /// Start automatic motor parameter calibration
    StartCalibration(CalibrationRequest),
    /// Stop/abort ongoing calibration
    StopCalibration,
    /// Calibration status update (Joint → Arm)
    CalibrationStatus(CalibrationStatus),
    /// Calibration final result (Joint → Arm)
    CalibrationResult(CalibrationResult),
}
```

### New Lifecycle State

```rust
pub enum LifecycleState {
    Unconfigured,
    Inactive,
    Active,
    Calibrating,  // NEW: During automatic calibration
    Error,
}
```

State transitions:
- `Inactive` → `Calibrating` (via StartCalibration)
- `Active` → `Calibrating` (via StartCalibration)
- `Calibrating` → `Inactive` (on success)
- `Calibrating` → `Error` (on failure)
- `Calibrating` → Previous state (via StopCalibration)

---

## Command Examples

### Start Calibration

```rust
// Full calibration (all phases)
let request = CalibrationRequest {
    phases: 0b11111,  // All 5 phases
    max_current: 8.0,  // 8A limit
    max_velocity: 5.0,  // 5 rad/s limit
    max_position_range: 3.14,  // ±180° from start
    phase_timeout: 60.0,  // 60s per phase
    return_home: true,
};

let msg = Message {
    header: Header {
        source_id: 0x0000,  // Arm
        target_id: 0x0010,  // Joint
        msg_id: 42,
    },
    payload: Payload::StartCalibration(request),
};

// Joint responds with Ack, then starts sending CalibrationStatus
```

### Receive Status Updates

```rust
// Joint sends status every 100ms during calibration
match msg.payload {
    Payload::CalibrationStatus(status) => {
        println!("Phase: {:?}", status.phase);
        println!("Progress: {:.1}%", status.progress * 100.0);
        println!("ETA: {:.1}s", status.time_remaining);
    }
    _ => {}
}
```

### Receive Final Result

```rust
match msg.payload {
    Payload::CalibrationResult(result) => {
        if result.success {
            println!("✅ Calibration complete!");
            println!("J = {:.6} kg·m²", result.parameters.inertia_J);
            println!("kt = {:.4} Nm/A", result.parameters.torque_constant_kt);
            println!("τ_c = {:.4} Nm", result.parameters.friction_coulomb);
            println!("Confidence: {:.1}%", result.confidence.overall * 100.0);
        } else {
            println!("❌ Calibration failed: error {}", result.error_code);
        }
    }
    _ => {}
}
```

---

## Firmware Implementation

### High-Level Architecture

```
┌─────────────────────────────────────────────────────┐
│              Calibration Manager                     │
│  - State machine (Idle → Test → Result)            │
│  - Test sequence coordination                        │
│  - Safety monitoring                                 │
│  - Result aggregation                                │
└──────────────┬──────────────────────────────────────┘
               │
    ┌──────────┴──────────┬────────────────┬─────────┐
    │                     │                │         │
┌───▼────┐  ┌─────────────▼────┐  ┌───────▼───────┐ │
│Inertia │  │  Friction Test   │  │ Torque Const  │ │
│  Test  │  │ - Velocity sweep │  │  Verification │ │
│        │  │ - Stribeck fit   │  │               │ │
└────────┘  └──────────────────┘  └───────────────┘ │
                                                     │
┌──────────────────────────────────────────────────┘
│
▼
┌────────────────────────────────────────────────────┐
│         Parameter Estimation & Validation          │
│  - Least squares fitting                           │
│  - Outlier rejection                               │
│  - Confidence calculation                          │
└────────────────────────────────────────────────────┘
```

### File Structure

```
joint_firmware/
├── src/
│   ├── calibration/
│   │   ├── mod.rs                    # Calibration manager
│   │   ├── inertia_test.rs           # Phase 1: Inertia ID
│   │   ├── friction_test.rs          # Phase 2: Friction ID
│   │   ├── torque_constant_test.rs   # Phase 3: kt verification
│   │   ├── damping_test.rs           # Phase 4: Damping ID
│   │   ├── validation_test.rs        # Phase 5: Validation
│   │   ├── estimator.rs              # Parameter estimation
│   │   └── safety.rs                 # Safety monitoring
│   ├── control/
│   │   └── motor_model.rs            # Use calibrated params here
│   └── main.rs
```

### Key Implementation Points

1. **Safety First:**
   - Monitor position limits at all times
   - Monitor current limits
   - Monitor temperature
   - Emergency stop on any violation
   - Return to safe position if aborted

2. **Robust Estimation:**
   - Multiple measurements per parameter
   - Outlier rejection (reject samples > 2σ from mean)
   - Confidence scoring based on variance
   - Model validation before accepting results

3. **Real-time Constraints:**
   - Run tests at control frequency (10 kHz)
   - Send status updates at human-readable rate (10 Hz)
   - Non-blocking: allow StopCalibration at any time

4. **Memory Efficiency:**
   - Use fixed-size buffers for measurements
   - Streaming parameter estimation (online updates)
   - No heap allocation during tests

---

## Error Codes

| Code | Name | Description |
|------|------|-------------|
| 0 | SUCCESS | Calibration completed successfully |
| 1 | POSITION_LIMIT | Position exceeded safe range |
| 2 | VELOCITY_LIMIT | Velocity exceeded limit |
| 3 | CURRENT_LIMIT | Current exceeded limit |
| 4 | TEMPERATURE_LIMIT | Motor overheated |
| 5 | TIMEOUT | Phase timeout exceeded |
| 6 | INVALID_STATE | Cannot calibrate in current state |
| 7 | CONVERGENCE_FAILED | Parameter estimation did not converge |
| 8 | LOW_CONFIDENCE | Results have low confidence (< 50%) |
| 9 | USER_ABORT | Calibration stopped by user |
| 10 | HARDWARE_ERROR | Encoder or motor driver error |

---

## Usage Example (Python)

```python
import irpc

# Connect to joint
bus = irpc.CANBus("/dev/ttyUSB0")
joint = irpc.Joint(bus, device_id=0x0010)

# Configure calibration
config = irpc.CalibrationRequest(
    phases=0b11111,  # All phases
    max_current=8.0,
    max_velocity=5.0,
    max_position_range=3.14,
    phase_timeout=60.0,
    return_home=True,
)

# Start calibration
print("🔧 Starting motor calibration...")
joint.start_calibration(config)

# Monitor progress
while True:
    status = joint.get_calibration_status()
    
    if status.phase == CalibrationPhase.Complete:
        break
    elif status.phase == CalibrationPhase.Failed:
        print("❌ Calibration failed!")
        break
    
    print(f"[{status.phase.name}] {status.progress*100:.1f}% "
          f"(ETA: {status.time_remaining:.1f}s)")
    time.sleep(0.5)

# Get results
result = joint.get_calibration_result()

if result.success:
    print("\n✅ Calibration Complete!")
    print(f"\n📊 Motor Parameters:")
    print(f"  J  = {result.parameters.inertia_J:.6f} kg·m²")
    print(f"  kt = {result.parameters.torque_constant_kt:.4f} Nm/A")
    print(f"  b  = {result.parameters.damping_b:.6f} Nm·s/rad")
    print(f"  τ_c = {result.parameters.friction_coulomb:.4f} Nm")
    print(f"  τ_s = {result.parameters.friction_stribeck:.4f} Nm")
    print(f"  v_s = {result.parameters.friction_vstribeck:.3f} rad/s")
    print(f"  b_f = {result.parameters.friction_viscous:.6f} Nm·s/rad")
    
    print(f"\n🎯 Confidence:")
    print(f"  Overall: {result.confidence.overall*100:.1f}%")
    print(f"  Inertia: {result.confidence.inertia*100:.1f}%")
    print(f"  Friction: {result.confidence.friction*100:.1f}%")
    
    print(f"\n⏱️  Total time: {result.total_time:.1f}s")
    
    # Save to file
    joint.save_parameters("motor_params.json")
else:
    print(f"❌ Calibration failed with error code {result.error_code}")
```

---

## Testing Strategy

### Unit Tests (Rust)
```rust
#[test]
fn test_inertia_calculation() {
    let torque = 0.3; // Nm
    let acceleration = 300.0; // rad/s²
    let kt = 0.15; // Nm/A
    
    let J = calculate_inertia(torque, acceleration);
    assert!((J - 0.001).abs() < 1e-6);  // Expect 0.001 kg·m²
}

#[test]
fn test_friction_model_fit() {
    let velocities = vec![0.5, 1.0, 2.0, 4.0];
    let torques = vec![0.025, 0.023, 0.030, 0.050];
    
    let params = fit_stribeck_model(&velocities, &torques);
    
    assert!(params.tau_coulomb > 0.0);
    assert!(params.tau_stribeck > 0.0);
    assert!(params.r_squared > 0.8);  // Good fit
}
```

### Integration Tests (Renode)
```robot
*** Test Cases ***
Test Full Calibration
    [Documentation]    Test complete calibration sequence
    [Tags]    integration    calibration
    
    # Send calibration request
    Send Calibration Request    max_current=5.0    max_velocity=3.0
    
    # Wait for completion (max 120s)
    ${result}=    Wait For Calibration Complete    timeout=120
    
    # Verify results
    Should Be Equal    ${result.success}    True
    Should Be True    ${result.parameters.inertia_J} > 0.0005
    Should Be True    ${result.parameters.inertia_J} < 0.0020
    Should Be True    ${result.confidence.overall} > 0.7

Test Calibration Abort
    [Documentation]    Test abort during calibration
    [Tags]    integration    calibration
    
    Send Calibration Request
    Sleep    2s
    Send Stop Calibration
    
    ${status}=    Get Calibration Status
    Should Be Equal    ${status.phase}    IDLE

Test Position Limit Safety
    [Documentation]    Test safety shutdown on limit violation
    [Tags]    safety    calibration
    
    # Configure with very small position range
    Send Calibration Request    max_position_range=0.1
    
    ${result}=    Wait For Calibration Complete
    
    Should Be Equal    ${result.success}    False
    Should Be Equal    ${result.error_code}    1  # POSITION_LIMIT
```

---

## Performance Targets

| Metric | Target | Notes |
|--------|--------|-------|
| Total calibration time | 50-80s | Depends on phases enabled |
| Inertia accuracy | ±10% | Compared to datasheet |
| Friction model R² | >0.85 | Good fit quality |
| Overall confidence | >70% | For production use |
| Position safety margin | 10° | From configured limits |
| Current safety margin | 20% | From configured limits |

---

## Future Enhancements (v3.0)

1. **Online Adaptation:**
   - Continuously update friction model during operation
   - Detect load changes in real-time
   - Adapt control gains automatically

2. **Temperature Compensation:**
   - Calibrate at multiple temperatures
   - Build temperature-dependent friction model
   - Adjust kt(T), τ_c(T) in real-time

3. **Load Identification:**
   - Identify external load parameters
   - Differentiate motor vs. load inertia
   - Compensate for payload variations

4. **Multi-Joint Calibration:**
   - Coordinated calibration of robot arm
   - Identify joint coupling effects
   - Optimize for multi-DOF dynamics

---

## References

1. **System Identification:**
   - Ljung, L. (1999). *System Identification: Theory for the User*
   - Åström, K. J., & Hägglund, T. (2006). *Advanced PID Control*

2. **Friction Modeling:**
   - Olsson, H., et al. (1998). *Friction Models and Friction Compensation*
   - Armstrong-Hélouvry, B. (1991). *Control of Machines with Friction*

3. **Motor Parameter Identification:**
   - Underwood, S. J., & Husain, I. (2010). *Online Parameter Estimation*
   - Vas, P. (1998). *Sensorless Vector and Direct Torque Control*

---

**Status:** Ready for Implementation  
**Next Step:** Implement in iRPC and joint_firmware in parallel

