use embassy_time::{Duration, Ticker};
use fixed::types::{I16F16, I1F15};

use crate::firmware::config::MotorConfig;
use crate::firmware::drivers::adc::CurrentSensors;
use crate::firmware::drivers::pwm::PhasePwm;
use crate::firmware::drivers::sensors::AngleSensor;
use crate::firmware::hardware::cordic::CordicEngine;
use crate::firmware::hardware::fmac::DualPiController;

/// FOC control loop frequency in Hz.
pub const FOC_LOOP_FREQ_HZ: u32 = 10_000;

/// FOC control loop period in microseconds.
pub const FOC_LOOP_PERIOD_US: u64 = 1_000_000 / FOC_LOOP_FREQ_HZ as u64;

/// FOC state machine states.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum FocState {
    Idle,
    Calibrating,
    Running,
    Fault,
}

/// FOC controller state.
pub struct FocController {
    state: FocState,
    adc_offsets: [u16; 2],
    target_current_dq: (I16F16, I16F16),
}

impl FocController {
    pub fn new() -> Self {
        Self {
            state: FocState::Idle,
            adc_offsets: [2048, 2048], // Default mid-scale
            target_current_dq: (I16F16::ZERO, I16F16::ZERO),
        }
    }

    /// Calibrate ADC offsets at zero current.
    pub async fn calibrate(&mut self, adc: &mut CurrentSensors<'_>) {
        self.state = FocState::Calibrating;
        self.adc_offsets = adc.calibrate_current_offsets(100).await;
        defmt::info!("ADC calibration complete: offsets={:?}", self.adc_offsets);
    }

    /// Set target D and Q axis currents.
    pub fn set_target_dq(&mut self, d: I16F16, q: I16F16) {
        self.target_current_dq = (d, q);
    }

    /// Run one FOC control iteration.
    pub fn update(
        &mut self,
        _adc: &mut CurrentSensors<'_>,
        encoder: &mut AngleSensor,
        pwm: &mut PhasePwm,
        cordic: &mut CordicEngine,
        pi_controller: &mut DualPiController,
        motor_config: &MotorConfig,
    ) {
        // 1. Read rotor angle
        let angle_mdeg = match encoder.read_electrical_angle(motor_config.pole_pairs) {
            Ok(angle) => angle,
            Err(_) => {
                self.state = FocState::Fault;
                pwm.disable();
                return;
            }
        };

        // 2. Clarke transform (ABC → αβ)
        // Placeholder: would read from ADC in real implementation
        let i_alpha = I1F15::ZERO;
        let i_beta = I1F15::ZERO;

        // 3. Park transform (αβ → dq)
        let (i_d, i_q) = cordic.park_transform(i_alpha, i_beta, angle_mdeg);

        // 4. PI controllers for current
        let (target_d, target_q) = self.target_current_dq;
        let error_d = target_d - I16F16::from_num(i_d.to_num::<f32>());
        let error_q = target_q - I16F16::from_num(i_q.to_num::<f32>());
        
        let dt = 1.0 / FOC_LOOP_FREQ_HZ as f32;
        let (v_d, v_q) = pi_controller.update(error_d, error_q, dt);

        // 5. Inverse Park transform (dq → αβ)
        let v_d_norm = I1F15::from_num(v_d.to_num::<f32>().clamp(-1.0, 1.0));
        let v_q_norm = I1F15::from_num(v_q.to_num::<f32>().clamp(-1.0, 1.0));
        let (v_alpha, v_beta) = cordic.inverse_park_transform(v_d_norm, v_q_norm, angle_mdeg);

        // 6. Space Vector PWM (αβ → PWM duties)
        let duties = self.svpwm(v_alpha, v_beta, pwm.max_duty());

        // 7. Update PWM outputs
        pwm.set_phase_duties(duties);
    }

    /// Space Vector PWM implementation.
    fn svpwm(&self, v_alpha: I1F15, v_beta: I1F15, max_duty: u16) -> [u16; 3] {
        let v_a = v_alpha.to_num::<f32>();
        let v_b = (-0.5 * v_a + 0.866 * v_beta.to_num::<f32>()) as f32;
        let v_c = (-0.5 * v_a - 0.866 * v_beta.to_num::<f32>()) as f32;

        let duty_a = ((v_a + 1.0) / 2.0).clamp(0.0, 1.0);
        let duty_b = ((v_b + 1.0) / 2.0).clamp(0.0, 1.0);
        let duty_c = ((v_c + 1.0) / 2.0).clamp(0.0, 1.0);

        [
            (duty_a * max_duty as f32) as u16,
            (duty_b * max_duty as f32) as u16,
            (duty_c * max_duty as f32) as u16,
        ]
    }
}

/// Main FOC control loop task.
#[embassy_executor::task]
pub async fn control_loop() {
    defmt::info!("FOC control loop starting");
    
    let mut ticker = Ticker::every(Duration::from_micros(FOC_LOOP_PERIOD_US));
    let mut iteration = 0u32;

    loop {
        ticker.next().await;
        
        iteration = iteration.wrapping_add(1);
        
        if iteration % 10_000 == 0 {
            defmt::info!("FOC loop: {} iterations", iteration);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foc_loop_timing() {
        assert_eq!(FOC_LOOP_FREQ_HZ, 10_000);
        assert_eq!(FOC_LOOP_PERIOD_US, 100);
    }

    #[test]
    fn svpwm_zero_voltage() {
        let controller = FocController::new();
        let duties = controller.svpwm(I1F15::ZERO, I1F15::ZERO, 1000);
        
        for duty in duties {
            assert!(duty >= 450 && duty <= 550);
        }
    }
}
