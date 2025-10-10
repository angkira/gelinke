//! Inertia identification test
//!
//! Method: Apply step torque, measure acceleration
//! Equation: J = τ / α = (kt * i_q) / α

use defmt::Format;
use super::super::types::{MeasurementBuffer, MeasurementSample, InertiaResult};

const NUM_TRIALS: usize = 5;
const SAMPLES_PER_TRIAL: usize = 100;

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

                    defmt::info!("Trial {}: J = {} kg·m²", self.trial + 1, J);

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

        defmt::info!("Inertia: J = {} ± {} kg·m² (CV = {}%)",
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
