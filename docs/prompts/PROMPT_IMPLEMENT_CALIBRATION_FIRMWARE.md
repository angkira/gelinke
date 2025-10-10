# PROMPT: Implement Motor Calibration in Firmware

**Task:** Implement automatic motor parameter calibration in STM32 firmware

**Context:** The iRPC protocol now supports calibration commands. Your task is to implement the actual calibration tests, parameter estimation, and safety monitoring in the embedded firmware.

---

## Architecture Overview

```
┌────────────────────────────────────────────────────────┐
│              Calibration Manager (FSM)                  │
│  - Receives StartCalibration command via iRPC          │
│  - Coordinates test execution                           │
│  - Monitors safety limits                               │
│  - Sends status updates (10 Hz)                         │
│  - Returns final results                                │
└──────────────────┬─────────────────────────────────────┘
                   │
        ┌──────────┴──────────┬──────────────┬───────────┐
        │                     │              │           │
┌───────▼────────┐  ┌─────────▼────────┐  ┌─▼──────────┐ │
│ Inertia Test   │  │  Friction Test   │  │ Validation │ │
│ - Step torque  │  │ - Vel sweep      │  │ - Tracking │ │
│ - Measure α    │  │ - Model fit      │  │ - RMS calc │ │
│ - Calc J       │  │ - R² score       │  │            │ │
└────────────────┘  └──────────────────┘  └────────────┘ │
                                                          │
┌─────────────────────────────────────────────────────────┘
│
▼
┌────────────────────────────────────────────────────────┐
│            Parameter Estimator                          │
│  - Least squares regression                             │
│  - Outlier rejection (2σ)                               │
│  - Confidence calculation                               │
└────────────────────────────────────────────────────────┘
```

---

## File Structure

Create these files in `joint_firmware/src/`:

```
src/
├── calibration/
│   ├── mod.rs                    # Main manager + FSM
│   ├── config.rs                 # Configuration struct
│   ├── tests/
│   │   ├── mod.rs
│   │   ├── inertia.rs            # Phase 1: Inertia ID
│   │   ├── friction.rs           # Phase 2: Friction ID
│   │   ├── torque_constant.rs    # Phase 3: kt verification
│   │   ├── damping.rs            # Phase 4: Damping ID (optional)
│   │   └── validation.rs         # Phase 5: Validation test
│   ├── estimator.rs              # Parameter estimation algorithms
│   ├── safety.rs                 # Safety monitor
│   └── types.rs                  # Internal data structures
└── main.rs                        # Wire up to iRPC handler
```

---

## Step 1: Types and Configuration

**File:** `src/calibration/types.rs`

```rust
//! Internal calibration types

use defmt::Format;

/// Calibration state machine
#[derive(Debug, Format, Clone, Copy, PartialEq, Eq)]
pub enum CalibrationState {
    Idle,
    Initializing,
    InertiaTest,
    FrictionTest,
    TorqueConstantTest,
    DampingTest,
    Validation,
    Finalizing,
    Complete,
    Failed(ErrorCode),
}

/// Error codes matching iRPC spec
#[derive(Debug, Format, Clone, Copy, PartialEq, Eq)]
#[repr(u16)]
pub enum ErrorCode {
    Success = 0,
    PositionLimit = 1,
    VelocityLimit = 2,
    CurrentLimit = 3,
    TemperatureLimit = 4,
    Timeout = 5,
    InvalidState = 6,
    ConvergenceFailed = 7,
    LowConfidence = 8,
    UserAbort = 9,
    HardwareError = 10,
}

/// Measurement sample for parameter estimation
#[derive(Clone, Copy)]
pub struct MeasurementSample {
    pub timestamp: f32,      // seconds
    pub position: f32,       // rad
    pub velocity: f32,       // rad/s
    pub acceleration: f32,   // rad/s²
    pub current_iq: f32,     // A
    pub temperature: f32,    // °C
}

/// Fixed-size buffer for measurements (no heap allocation)
pub struct MeasurementBuffer<const N: usize> {
    samples: [MeasurementSample; N],
    count: usize,
}

impl<const N: usize> MeasurementBuffer<N> {
    pub const fn new() -> Self {
        Self {
            samples: [MeasurementSample {
                timestamp: 0.0,
                position: 0.0,
                velocity: 0.0,
                acceleration: 0.0,
                current_iq: 0.0,
                temperature: 0.0,
            }; N],
            count: 0,
        }
    }

    pub fn push(&mut self, sample: MeasurementSample) -> Result<(), ()> {
        if self.count < N {
            self.samples[self.count] = sample;
            self.count += 1;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn samples(&self) -> &[MeasurementSample] {
        &self.samples[..self.count]
    }

    pub fn clear(&mut self) {
        self.count = 0;
    }

    pub fn len(&self) -> usize {
        self.count
    }
}

/// Intermediate test results
pub struct TestResults {
    pub inertia: Option<InertiaResult>,
    pub friction: Option<FrictionResult>,
    pub torque_constant: Option<f32>,
    pub damping: Option<f32>,
}

pub struct InertiaResult {
    pub J: f32,                 // kg·m²
    pub variance: f32,          // Measurement variance
    pub confidence: f32,        // 0.0 - 1.0
}

pub struct FrictionResult {
    pub tau_coulomb: f32,       // Nm
    pub tau_stribeck: f32,      // Nm
    pub v_stribeck: f32,        // rad/s
    pub b_viscous: f32,         // Nm·s/rad
    pub r_squared: f32,         // Model fit quality
    pub confidence: f32,        // 0.0 - 1.0
}
```

**File:** `src/calibration/config.rs`

```rust
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
```

---

## Step 2: Safety Monitor

**File:** `src/calibration/safety.rs`

```rust
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
            defmt::error!("Position limit exceeded: {:.3} rad", position_error);
            return Err(ErrorCode::PositionLimit);
        }

        // Velocity limit
        if sample.velocity.abs() > self.config.max_velocity * 1.1 {
            defmt::error!("Velocity limit exceeded: {:.3} rad/s", sample.velocity);
            return Err(ErrorCode::VelocityLimit);
        }

        // Current limit
        if sample.current_iq.abs() > self.config.max_current * 1.1 {
            defmt::error!("Current limit exceeded: {:.3} A", sample.current_iq);
            return Err(ErrorCode::CurrentLimit);
        }

        // Temperature limit (hard-coded 80°C)
        if sample.temperature > 80.0 {
            defmt::error!("Temperature limit exceeded: {:.1} °C", sample.temperature);
            return Err(ErrorCode::TemperatureLimit);
        }

        // Phase timeout
        let phase_duration = sample.timestamp - self.phase_start_time;
        if phase_duration > self.config.phase_timeout {
            defmt::error!("Phase timeout: {:.1}s > {:.1}s", phase_duration, self.config.phase_timeout);
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
```

---

## Step 3: Inertia Test Implementation

**File:** `src/calibration/tests/inertia.rs`

```rust
//! Inertia identification test
//!
//! Method: Apply step torque, measure acceleration
//! Equation: J = τ / α = (kt * i_q) / α

use defmt::Format;
use super::super::types::{MeasurementBuffer, MeasurementSample, InertiaResult, ErrorCode};

const NUM_TRIALS: usize = 5;
const SAMPLES_PER_TRIAL: usize = 100;

#[derive(Format)]
pub struct InertiaTest {
    trial: usize,
    phase: TestPhase,
    start_time: f32,
    measurements: MeasurementBuffer<SAMPLES_PER_TRIAL>,
    results: [f32; NUM_TRIALS],  // J estimates from each trial
    kt_nominal: f32,  // Torque constant (from config or default)
}

#[derive(Format, PartialEq)]
enum TestPhase {
    Idle,
    Accelerating,
    WaitSettle,
}

impl InertiaTest {
    pub fn new(kt_nominal: f32) -> Self {
        Self {
            trial: 0,
            phase: TestPhase::Idle,
            start_time: 0.0,
            measurements: MeasurementBuffer::new(),
            results: [0.0; NUM_TRIALS],
            kt_nominal,
        }
    }

    /// Get current commanded i_q for this trial
    pub fn get_command_current(&self) -> f32 {
        // Use different currents for robustness: 1A, 2A, 3A, 4A, 5A
        (self.trial + 1) as f32
    }

    /// Update state machine, returns (i_q_command, is_complete)
    pub fn update(&mut self, sample: MeasurementSample) -> (f32, bool) {
        match self.phase {
            TestPhase::Idle => {
                // Start new trial
                self.start_time = sample.timestamp;
                self.measurements.clear();
                self.phase = TestPhase::Accelerating;
                (self.get_command_current(), false)
            }
            
            TestPhase::Accelerating => {
                // Collect measurements during acceleration (1 second)
                let _ = self.measurements.push(sample);
                
                if sample.timestamp - self.start_time > 1.0 {
                    // End of acceleration phase
                    self.phase = TestPhase::WaitSettle;
                    self.start_time = sample.timestamp;
                }
                
                (self.get_command_current(), false)
            }
            
            TestPhase::WaitSettle => {
                // Wait for system to settle (0.5 seconds)
                if sample.timestamp - self.start_time > 0.5 {
                    // Calculate J for this trial
                    let J = self.estimate_inertia_for_trial();
                    self.results[self.trial] = J;
                    
                    defmt::info!("Trial {}: J = {:.6} kg·m²", self.trial + 1, J);
                    
                    self.trial += 1;
                    
                    if self.trial >= NUM_TRIALS {
                        return (0.0, true);  // Test complete
                    }
                    
                    // Start next trial
                    self.phase = TestPhase::Idle;
                }
                
                (0.0, false)  // Zero current during settle
            }
        }
    }

    fn estimate_inertia_for_trial(&self) -> f32 {
        let samples = self.measurements.samples();
        
        // Calculate average acceleration during test
        let mut sum_accel = 0.0;
        let mut count = 0;
        
        for sample in samples {
            if sample.acceleration.abs() > 1.0 {  // Filter out near-zero values
                sum_accel += sample.acceleration;
                count += 1;
            }
        }
        
        if count == 0 {
            return 0.0;
        }
        
        let avg_acceleration = sum_accel / count as f32;
        let i_q = self.get_command_current();
        let torque = self.kt_nominal * i_q;
        
        // J = τ / α
        let J = torque / avg_acceleration;
        
        J
    }

    /// Get final result with confidence
    pub fn get_result(&self) -> InertiaResult {
        // Calculate mean
        let mut sum = 0.0;
        for &J in &self.results {
            sum += J;
        }
        let mean = sum / NUM_TRIALS as f32;
        
        // Calculate variance
        let mut var_sum = 0.0;
        for &J in &self.results {
            let diff = J - mean;
            var_sum += diff * diff;
        }
        let variance = var_sum / NUM_TRIALS as f32;
        let std_dev = libm::sqrtf(variance);
        
        // Confidence based on coefficient of variation (lower is better)
        let cv = std_dev / mean;
        let confidence = if cv < 0.05 {
            1.0  // Excellent
        } else if cv < 0.10 {
            0.9  // Good
        } else if cv < 0.20 {
            0.7  // Acceptable
        } else {
            0.5  // Poor
        };
        
        defmt::info!("Inertia: J = {:.6} ± {:.6} kg·m² (CV = {:.1}%)", 
                     mean, std_dev, cv * 100.0);
        
        InertiaResult {
            J: mean,
            variance,
            confidence,
        }
    }

    pub fn get_progress(&self) -> f32 {
        self.trial as f32 / NUM_TRIALS as f32
    }
}
```

---

## Step 4: Friction Test Implementation

**File:** `src/calibration/tests/friction.rs`

```rust
//! Friction identification test
//!
//! Method: Constant velocity tracking at multiple speeds
//! At steady state: τ_friction = kt * i_q
//! Fit Stribeck model: τ_f(v) = τ_c*sgn(v) + τ_s*exp(-(v/v_s)²) + b_f*v

use defmt::Format;
use super::super::types::{MeasurementBuffer, MeasurementSample, FrictionResult};

const NUM_VELOCITIES: usize = 8;
const SAMPLES_PER_VELOCITY: usize = 300;  // 3s at 100Hz

#[derive(Format)]
pub struct FrictionTest {
    velocity_index: usize,
    phase: TestPhase,
    start_time: f32,
    measurements: MeasurementBuffer<SAMPLES_PER_VELOCITY>,
    velocity_setpoints: [f32; NUM_VELOCITIES],
    friction_estimates: [f32; NUM_VELOCITIES],
    kt_nominal: f32,
}

#[derive(Format, PartialEq)]
enum TestPhase {
    Ramping,
    Steady,
}

impl FrictionTest {
    pub fn new(kt_nominal: f32, max_velocity: f32) -> Self {
        // Test velocities: 0.5, 1.0, 2.0, 4.0 rad/s (positive and negative)
        let velocities = [
            max_velocity * 0.1,
            max_velocity * 0.2,
            max_velocity * 0.4,
            max_velocity * 0.8,
            -max_velocity * 0.1,
            -max_velocity * 0.2,
            -max_velocity * 0.4,
            -max_velocity * 0.8,
        ];
        
        Self {
            velocity_index: 0,
            phase: TestPhase::Ramping,
            start_time: 0.0,
            measurements: MeasurementBuffer::new(),
            velocity_setpoints: velocities,
            friction_estimates: [0.0; NUM_VELOCITIES],
            kt_nominal,
        }
    }

    pub fn update(&mut self, sample: MeasurementSample) -> (f32, bool) {
        let target_vel = self.velocity_setpoints[self.velocity_index];
        
        match self.phase {
            TestPhase::Ramping => {
                // Ramp to target velocity (1 second)
                if sample.timestamp - self.start_time > 1.0 {
                    self.phase = TestPhase::Steady;
                    self.start_time = sample.timestamp;
                    self.measurements.clear();
                }
                (target_vel, false)
            }
            
            TestPhase::Steady => {
                // Hold velocity and collect measurements
                let _ = self.measurements.push(sample);
                
                if sample.timestamp - self.start_time > 3.0 {
                    // Estimate friction for this velocity
                    let tau_friction = self.estimate_friction_at_velocity();
                    self.friction_estimates[self.velocity_index] = tau_friction;
                    
                    defmt::info!("Velocity {}: v = {:.2} rad/s, τ_f = {:.4} Nm", 
                                 self.velocity_index + 1, target_vel, tau_friction);
                    
                    self.velocity_index += 1;
                    
                    if self.velocity_index >= NUM_VELOCITIES {
                        return (0.0, true);  // Test complete
                    }
                    
                    // Next velocity
                    self.phase = TestPhase::Ramping;
                    self.start_time = sample.timestamp;
                }
                
                (target_vel, false)
            }
        }
    }

    fn estimate_friction_at_velocity(&self) -> f32 {
        let samples = self.measurements.samples();
        
        // Average i_q during steady state
        let mut sum_iq = 0.0;
        for sample in samples {
            sum_iq += sample.current_iq;
        }
        let avg_iq = sum_iq / samples.len() as f32;
        
        // Friction torque = kt * i_q (at steady state, no acceleration)
        let tau_friction = self.kt_nominal * avg_iq;
        
        tau_friction
    }

    pub fn get_result(&self) -> FrictionResult {
        // Fit Stribeck model using least squares
        // Simplified: just fit Coulomb + viscous for now
        // τ_f = τ_c * sgn(v) + b_f * v
        
        // Separate positive and negative velocities
        let mut tau_pos = Vec::new();
        let mut vel_pos = Vec::new();
        let mut tau_neg = Vec::new();
        let mut vel_neg = Vec::new();
        
        for i in 0..NUM_VELOCITIES {
            let v = self.velocity_setpoints[i];
            let tau = self.friction_estimates[i];
            
            if v > 0.0 {
                vel_pos.push(v);
                tau_pos.push(tau);
            } else {
                vel_neg.push(-v);
                tau_neg.push(-tau);
            }
        }
        
        // Estimate Coulomb friction (intercept)
        let tau_c_pos = tau_pos.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&0.0);
        let tau_c_neg = tau_neg.iter().min_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&0.0);
        let tau_coulomb = (tau_c_pos + tau_c_neg) / 2.0;
        
        // Estimate viscous friction (slope)
        // Simple: (τ_max - τ_min) / (v_max - v_min)
        let tau_max = *tau_pos.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&0.0);
        let tau_min = tau_coulomb;
        let v_max = *vel_pos.iter().max_by(|a, b| a.partial_cmp(b).unwrap()).unwrap_or(&1.0);
        let b_viscous = (tau_max - tau_min) / v_max;
        
        // For now, set Stribeck params to defaults (full fit is complex)
        let tau_stribeck = tau_coulomb * 0.3;  // Typical: 30% of Coulomb
        let v_stribeck = 0.1;  // Typical: 0.1 rad/s
        
        // Calculate R² (model fit quality)
        // TODO: Implement proper R² calculation
        let r_squared = 0.85;  // Placeholder
        
        let confidence = if r_squared > 0.90 {
            0.95
        } else if r_squared > 0.80 {
            0.85
        } else {
            0.70
        };
        
        defmt::info!("Friction: τ_c = {:.4} Nm, b_f = {:.6} Nm·s/rad", 
                     tau_coulomb, b_viscous);
        
        FrictionResult {
            tau_coulomb,
            tau_stribeck,
            v_stribeck,
            b_viscous,
            r_squared,
            confidence,
        }
    }

    pub fn get_progress(&self) -> f32 {
        self.velocity_index as f32 / NUM_VELOCITIES as f32
    }
}
```

---

## Step 5: Main Calibration Manager

**File:** `src/calibration/mod.rs`

```rust
//! Motor parameter calibration manager

pub mod config;
pub mod types;
pub mod safety;
pub mod tests;
mod estimator;

use defmt::Format;
use config::{CalibrationConfig, PHASE_INERTIA, PHASE_FRICTION, PHASE_VALIDATION};
use types::{CalibrationState, ErrorCode, TestResults};
use safety::SafetyMonitor;
use tests::{inertia::InertiaTest, friction::FrictionTest};

// Re-export for iRPC integration
pub use types::MeasurementSample;

#[derive(Format)]
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
        
        defmt::info!("Calibration started at position {:.3} rad", current_position);
        
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
                }
                (0.0, 0.0, false)
            }
            
            CalibrationState::InertiaTest => {
                if let Some(ref mut test) = self.inertia_test {
                    let (i_q, complete) = test.update(sample);
                    
                    if complete {
                        let result = test.get_result();
                        self.results.inertia = Some(result);
                        
                        // Move to next phase
                        if self.config.phase_enabled(PHASE_FRICTION) {
                            self.friction_test = Some(FrictionTest::new(result.J, self.config.max_velocity));
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
```

---

## Step 6: Integration with Main Loop

**File:** `src/main.rs` (add to iRPC handler)

```rust
use calibration::{CalibrationManager, MeasurementSample};

// In main struct:
struct JointController {
    calibration: CalibrationManager,
    // ... other fields
}

// In iRPC message handler:
match payload {
    Payload::StartCalibration(request) => {
        let config = CalibrationConfig {
            phases: request.phases,
            max_current: request.max_current,
            max_velocity: request.max_velocity,
            max_position_range: request.max_position_range,
            phase_timeout: request.phase_timeout,
            return_home: request.return_home,
            home_position: 0.0,  // Will be set by manager
        };
        
        match controller.calibration.start(config, current_position, current_time) {
            Ok(()) => send_ack(msg_id),
            Err(error) => send_nack(msg_id, error as u16),
        }
    }
    
    Payload::StopCalibration => {
        controller.calibration.stop();
        send_ack(msg_id);
    }
    
    _ => {}
}

// In control loop (10 kHz):
if controller.calibration.state() != CalibrationState::Idle {
    let sample = MeasurementSample {
        timestamp: get_time(),
        position: encoder.position(),
        velocity: encoder.velocity(),
        acceleration: calculate_acceleration(),
        current_iq: foc.get_iq(),
        temperature: adc.temperature(),
    };
    
    let (i_q_cmd, vel_cmd, complete) = controller.calibration.update(sample);
    
    // Apply commands
    if i_q_cmd != 0.0 {
        foc.set_iq(i_q_cmd);
    } else if vel_cmd != 0.0 {
        velocity_controller.set_target(vel_cmd);
    }
    
    // Send status update (10 Hz)
    if status_timer.elapsed() > 100 {
        let status = controller.calibration.get_status(get_time());
        send_calibration_status(status);
        status_timer.reset();
    }
    
    // Check completion
    if complete {
        let result = controller.calibration.get_result(get_time());
        send_calibration_result(result);
        
        // Save parameters to flash if successful
        if result.success {
            flash_write_motor_params(&result.parameters);
        }
    }
}
```

---

## Step 7: Testing

**File:** `tests/calibration_tests.rs`

```rust
#[test]
fn test_calibration_manager_lifecycle() {
    let mut manager = CalibrationManager::new();
    
    let config = CalibrationConfig {
        phases: 0b00001,  // Inertia only
        max_current: 5.0,
        max_velocity: 3.0,
        max_position_range: 1.0,
        phase_timeout: 30.0,
        return_home: true,
        home_position: 0.0,
    };
    
    assert!(manager.start(config, 0.0, 0.0).is_ok());
    assert_eq!(manager.state(), CalibrationState::Initializing);
}

#[test]
fn test_safety_position_limit() {
    let config = CalibrationConfig {
        max_position_range: 0.5,
        ..Default::default()
    };
    
    let safety = SafetyMonitor::new(config, 0.0, 0.0);
    
    let sample = MeasurementSample {
        position: 0.6,  // Exceeds limit
        ..Default::default()
    };
    
    assert_eq!(safety.check_safety(&sample), Err(ErrorCode::PositionLimit));
}
```

---

## Validation

Before submitting:

- [ ] All modules compile without errors
- [ ] Unit tests pass
- [ ] Calibration runs in Renode simulation
- [ ] Status updates sent at 10 Hz
- [ ] Safety limits trigger correctly
- [ ] Results have reasonable values
- [ ] Memory usage < 10 KB
- [ ] No heap allocations during tests

---

## Expected Timeline

- Types & config: **2 hours**
- Safety monitor: **1 hour**
- Inertia test: **3 hours**
- Friction test: **4 hours**
- Manager & integration: **3 hours**
- Testing & debugging: **3 hours**

**Total:** ~16 hours

---

## Success Criteria

✅ Calibration completes all enabled phases  
✅ Safety limits prevent damage  
✅ Results match simulation (±20%)  
✅ iRPC status updates work  
✅ Can abort mid-calibration  
✅ Returns to home position  

---

**This is production-critical infrastructure. Take your time and test thoroughly! 🔩**

