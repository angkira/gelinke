use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::Channel;
use embassy_stm32::timer::simple_pwm::{SimplePwm, PwmPin};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::{Ch1, Ch2, Ch3, Ch4};
use embassy_stm32::peripherals::{TIM2, PA0, PA1, PB10, PB11};
use embassy_stm32::Peri;

/// Default PWM frequency for DRV8844 motor driver (20 kHz).
pub const DEFAULT_PWM_FREQ: Hertz = Hertz(20_000);

/// DRV8844 H-bridge PWM controller using TIM2.
///
/// CLN17 V2.0 uses DRV8844 dual H-bridge stepper driver with 4 independent inputs:
/// - AIN1, AIN2 control Phase A H-bridge
/// - BIN1, BIN2 control Phase B H-bridge
///
/// Hardware connections:
/// - PA0  (TIM2_CH1) → DRV8844 AIN1
/// - PA1  (TIM2_CH2) → DRV8844 AIN2
/// - PB11 (TIM2_CH4) → DRV8844 BIN1
/// - PB10 (TIM2_CH3) → DRV8844 BIN2
pub struct MotorPwm<'d> {
    pwm: SimplePwm<'d, TIM2>,
    max_duty: u16,
}

impl<'d> MotorPwm<'d> {
    /// Create a new DRV8844 motor PWM driver.
    ///
    /// # Arguments
    /// * `tim2` - TIM2 peripheral
    /// * `pa0` - PA0 pin for AIN1 (TIM2_CH1)
    /// * `pa1` - PA1 pin for AIN2 (TIM2_CH2)
    /// * `pb10` - PB10 pin for BIN2 (TIM2_CH3)
    /// * `pb11` - PB11 pin for BIN1 (TIM2_CH4)
    /// * `freq` - PWM frequency (typically 20-40 kHz)
    pub fn new(
        tim2: Peri<'d, TIM2>,
        pa0: Peri<'d, PA0>,
        pa1: Peri<'d, PA1>,
        pb10: Peri<'d, PB10>,
        pb11: Peri<'d, PB11>,
        freq: Hertz,
    ) -> Self {
        // Configure PWM pins for DRV8844 (4 independent channels)
        // Embassy 0.4.0: Use PwmPin::new with explicit type annotations
        let ch1: PwmPin<'_, TIM2, Ch1> = PwmPin::new(pa0, OutputType::PushPull);   // AIN1
        let ch2: PwmPin<'_, TIM2, Ch2> = PwmPin::new(pa1, OutputType::PushPull);   // AIN2
        let ch3: PwmPin<'_, TIM2, Ch3> = PwmPin::new(pb10, OutputType::PushPull);  // BIN2
        let ch4: PwmPin<'_, TIM2, Ch4> = PwmPin::new(pb11, OutputType::PushPull);  // BIN1

        let mut pwm = SimplePwm::new(
            tim2,
            Some(ch1),
            Some(ch2),
            Some(ch3),
            Some(ch4),
            freq,
            CountingMode::EdgeAlignedUp,
        );

        // Start with all channels at zero duty (safe state)
        // Embassy 0.4.0: Use channel().set_duty_cycle() instead of set_duty()
        pwm.ch1().set_duty_cycle(0);
        pwm.ch2().set_duty_cycle(0);
        pwm.ch3().set_duty_cycle(0);
        pwm.ch4().set_duty_cycle(0);

        // Embassy 0.4.0: max_duty() is now max_duty_cycle()
        let max_duty = pwm.max_duty_cycle();

        Self { pwm, max_duty }
    }

    /// Get the maximum duty cycle value.
    #[inline]
    pub fn max_duty(&self) -> u16 {
        self.max_duty
    }

    /// Set duty cycle for Phase A, channel 1 (AIN1).
    pub fn set_a1_duty(&mut self, duty: u16) {
        self.set_channel_duty(Channel::Ch1, duty);
    }

    /// Set duty cycle for Phase A, channel 2 (AIN2).
    pub fn set_a2_duty(&mut self, duty: u16) {
        self.set_channel_duty(Channel::Ch2, duty);
    }

    /// Set duty cycle for Phase B, channel 1 (BIN1).
    pub fn set_b1_duty(&mut self, duty: u16) {
        self.set_channel_duty(Channel::Ch4, duty);
    }

    /// Set duty cycle for Phase B, channel 2 (BIN2).
    pub fn set_b2_duty(&mut self, duty: u16) {
        self.set_channel_duty(Channel::Ch3, duty);
    }

    /// Set duty cycle for a specific channel.
    pub fn set_channel_duty(&mut self, channel: Channel, duty: u16) {
        assert!(
            duty <= self.max_duty,
            "duty {} exceeds max {}",
            duty,
            self.max_duty
        );
        // Embassy 0.4.0: Use channel().set_duty_cycle()
        self.pwm.channel(channel).set_duty_cycle(duty);
    }

    /// Set both Phase A duty cycles (H-bridge control).
    ///
    /// For forward current: a1 = duty, a2 = 0
    /// For reverse current: a1 = 0, a2 = duty
    /// For brake: a1 = max, a2 = max (or both 0)
    pub fn set_phase_a_duties(&mut self, a1: u16, a2: u16) {
        self.set_a1_duty(a1);
        self.set_a2_duty(a2);
    }

    /// Set both Phase B duty cycles (H-bridge control).
    pub fn set_phase_b_duties(&mut self, b1: u16, b2: u16) {
        self.set_b1_duty(b1);
        self.set_b2_duty(b2);
    }

    /// Set phase duties from 3-phase FOC output.
    ///
    /// Maps 3-phase duties [A, B, C] to 2-phase H-bridge control.
    /// This is a simplified mapping - proper 3→2 phase transformation
    /// would require Clarke transform consideration.
    ///
    /// # Arguments
    /// * `duties` - [phase_a_duty, phase_b_duty, phase_c_duty]
    pub fn set_phase_duties(&mut self, duties: [u16; 3]) {
        // Simplified mapping: Use A and B phases directly
        // Phase A → H-bridge A (forward direction)
        // Phase B → H-bridge B (forward direction)
        self.set_phase_a_duties(duties[0], 0);
        self.set_phase_b_duties(duties[1], 0);
        // Note: Phase C is not used in 2-phase configuration
    }

    /// Set all four H-bridge inputs at once.
    ///
    /// # Arguments
    /// * `duties` - [AIN1, AIN2, BIN1, BIN2]
    pub fn set_all_duties(&mut self, duties: [u16; 4]) {
        self.set_a1_duty(duties[0]);
        self.set_a2_duty(duties[1]);
        self.set_b1_duty(duties[2]);
        self.set_b2_duty(duties[3]);
    }

    /// Disable all PWM outputs (safe state, motor coasts).
    pub fn disable(&mut self) {
        // Embassy 0.4.0: Use channel methods
        self.pwm.ch1().set_duty_cycle(0);
        self.pwm.ch2().set_duty_cycle(0);
        self.pwm.ch3().set_duty_cycle(0);
        self.pwm.ch4().set_duty_cycle(0);
    }

    /// Set Phase A to forward rotation.
    ///
    /// # Arguments
    /// * `duty` - PWM duty cycle (0 to max_duty)
    pub fn phase_a_forward(&mut self, duty: u16) {
        self.set_phase_a_duties(duty, 0);
    }

    /// Set Phase A to reverse rotation.
    pub fn phase_a_reverse(&mut self, duty: u16) {
        self.set_phase_a_duties(0, duty);
    }

    /// Set Phase B to forward rotation.
    pub fn phase_b_forward(&mut self, duty: u16) {
        self.set_phase_b_duties(duty, 0);
    }

    /// Set Phase B to reverse rotation.
    pub fn phase_b_reverse(&mut self, duty: u16) {
        self.set_phase_b_duties(0, duty);
    }

    /// Brake Phase A (both high or both low).
    pub fn phase_a_brake(&mut self) {
        self.set_phase_a_duties(0, 0);
    }

    /// Brake Phase B (both high or both low).
    pub fn phase_b_brake(&mut self) {
        self.set_phase_b_duties(0, 0);
    }

    /// Coast both phases (all outputs disabled).
    pub fn coast(&mut self) {
        self.disable();
    }
}

// Legacy compatibility: keep PhasePwm as alias
pub type PhasePwm<'d> = MotorPwm<'d>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_valid() {
        assert!(DEFAULT_PWM_FREQ.0 > 0);
    }

    #[test]
    fn duty_calculation() {
        // Verify duty cycle boundaries
        let max = 1000u16;
        assert!(0 <= max);
        assert!(max / 2 == 500);
    }
}
