use embassy_stm32::gpio::OutputType;
use embassy_stm32::time::Hertz;
use embassy_stm32::timer::Channel;
use embassy_stm32::timer::complementary_pwm::{ComplementaryPwm, ComplementaryPwmPin};
use embassy_stm32::timer::low_level::CountingMode;
use embassy_stm32::timer::simple_pwm::PwmPin;
use embassy_stm32::{Peripherals, peripherals::TIM1};

/// Default PWM frequency for phase current control (20 kHz).
pub const DEFAULT_PWM_FREQ: Hertz = Hertz(20_000);

/// Default deadtime in timer ticks between complementary transitions.
pub const DEFAULT_DEADTIME_TICKS: u16 = 100;

/// Three-phase complementary PWM controller using TIM1.
pub struct PhasePwm<'d> {
    pwm: ComplementaryPwm<'d, TIM1>,
    max_duty: u16,
}

impl<'d> PhasePwm<'d> {
    /// Create a new three-phase PWM driver in centre-aligned mode.
    ///
    /// # Arguments
    /// * `p` - Peripherals struct from embassy_stm32::init()
    /// * `freq` - PWM frequency
    pub fn new(p: Peripherals, freq: Hertz) -> Self {
        // Configure PWM pins: TIM1 CH1/CH2/CH3 with complementary outputs
        let ch1 = PwmPin::new(p.PA8, OutputType::PushPull);
        let ch1n = ComplementaryPwmPin::new(p.PA7, OutputType::PushPull);
        let ch2 = PwmPin::new(p.PA9, OutputType::PushPull);
        let ch2n = ComplementaryPwmPin::new(p.PB0, OutputType::PushPull);
        let ch3 = PwmPin::new(p.PA10, OutputType::PushPull);
        let ch3n = ComplementaryPwmPin::new(p.PB1, OutputType::PushPull);

        let mut pwm = ComplementaryPwm::new(
            p.TIM1,
            Some(ch1),
            Some(ch1n),
            Some(ch2),
            Some(ch2n),
            Some(ch3),
            Some(ch3n),
            None,
            None,
            freq,
            CountingMode::CenterAlignedBothInterrupts,
        );

        // Configure dead-time to prevent shoot-through
        pwm.set_dead_time(DEFAULT_DEADTIME_TICKS);

        let max_duty = pwm.get_max_duty();

        Self { pwm, max_duty }
    }

    /// Get the maximum duty cycle value.
    #[inline]
    pub fn max_duty(&self) -> u16 {
        self.max_duty
    }

    /// Set duty cycle for a specific channel and enable it.
    pub fn set_channel_duty(&mut self, channel: Channel, duty: u16) {
        assert!(
            duty <= self.max_duty,
            "duty {} exceeds max {}",
            duty,
            self.max_duty
        );
        self.pwm.set_duty(channel, duty);
        self.pwm.enable(channel);
    }

    /// Set all three phase duty cycles at once.
    pub fn set_phase_duties(&mut self, duties: [u16; 3]) {
        self.set_channel_duty(Channel::Ch1, duties[0]);
        self.set_channel_duty(Channel::Ch2, duties[1]);
        self.set_channel_duty(Channel::Ch3, duties[2]);
    }

    /// Disable all PWM outputs (safe state).
    pub fn disable(&mut self) {
        self.pwm.disable(Channel::Ch1);
        self.pwm.disable(Channel::Ch2);
        self.pwm.disable(Channel::Ch3);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_valid() {
        assert!(DEFAULT_PWM_FREQ.0 > 0);
        assert!(DEFAULT_DEADTIME_TICKS > 0);
    }
}
