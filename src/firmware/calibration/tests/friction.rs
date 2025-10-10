//! Friction identification test
//!
//! Method: Constant velocity tracking at multiple speeds
//! At steady state: τ_friction = kt * i_q
//! Fit Stribeck model: τ_f(v) = τ_c*sgn(v) + τ_s*exp(-(v/v_s)²) + b_f*v

use defmt::Format;
use super::super::types::{MeasurementBuffer, MeasurementSample, FrictionResult};

const NUM_VELOCITIES: usize = 8;
const SAMPLES_PER_VELOCITY: usize = 300;  // 3s at 100Hz

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

                    defmt::info!("Velocity {}: v = {} rad/s, τ_f = {} Nm",
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
        let mut tau_pos_min = f32::MAX;
        let mut tau_pos_max = f32::MIN;
        let mut tau_neg_min = f32::MAX;
        let mut vel_pos_max = 0.0f32;

        for i in 0..NUM_VELOCITIES {
            let v = self.velocity_setpoints[i];
            let tau = self.friction_estimates[i];

            if v > 0.0 {
                if tau < tau_pos_min {
                    tau_pos_min = tau;
                }
                if tau > tau_pos_max {
                    tau_pos_max = tau;
                }
                if v > vel_pos_max {
                    vel_pos_max = v;
                }
            } else if v < 0.0 {
                let tau_abs = -tau;
                if tau_abs < tau_neg_min {
                    tau_neg_min = tau_abs;
                }
            }
        }

        // Estimate Coulomb friction (intercept)
        let tau_coulomb = (tau_pos_min + tau_neg_min) / 2.0;

        // Estimate viscous friction (slope)
        // Simple: (τ_max - τ_min) / (v_max - v_min)
        let tau_max = tau_pos_max;
        let tau_min = tau_coulomb;
        let v_max = if vel_pos_max > 0.0 { vel_pos_max } else { 1.0 };
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

        defmt::info!("Friction: τ_c = {} Nm, b_f = {} Nm·s/rad",
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
