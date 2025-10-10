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

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}

/// Intermediate test results
#[derive(Format)]
pub struct TestResults {
    pub inertia: Option<InertiaResult>,
    pub friction: Option<FrictionResult>,
    pub torque_constant: Option<f32>,
    pub damping: Option<f32>,
}

#[derive(Format)]
pub struct InertiaResult {
    pub J: f32,                 // kg·m²
    pub variance: f32,          // Measurement variance
    pub confidence: f32,        // 0.0 - 1.0
}

#[derive(Format)]
pub struct FrictionResult {
    pub tau_coulomb: f32,       // Nm
    pub tau_stribeck: f32,      // Nm
    pub v_stribeck: f32,        // rad/s
    pub b_viscous: f32,         // Nm·s/rad
    pub r_squared: f32,         // Model fit quality
    pub confidence: f32,        // 0.0 - 1.0
}
