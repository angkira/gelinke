/// Input Shaping for Vibration Suppression
///
/// Reduces residual vibrations in mechanical systems by convolving the command
/// signal with a series of impulses designed to cancel out resonant modes.
///
/// Key advantages:
/// - 50-99% vibration reduction
/// - Allows 30-50% higher speeds without overshoot
/// - No feedback required (feedforward technique)
/// - Robust to modeling errors (especially ZVD and EI)

use fixed::types::I16F16;
use heapless::Deque;

/// Maximum number of impulses in a shaper
const MAX_IMPULSES: usize = 4;

/// Maximum buffer size for command history
const MAX_BUFFER_SIZE: usize = 64;

/// Impulse in shaper sequence
#[derive(Clone, Copy, Debug)]
pub struct Impulse {
    /// Time delay for this impulse (seconds)
    pub time: I16F16,
    /// Amplitude of this impulse (0-1, sum = 1)
    pub amplitude: I16F16,
}

/// Input shaper type
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ShaperType {
    /// Zero Vibration (ZV) - 2 impulses, ±25% frequency robustness
    ZV,
    /// Zero Vibration Derivative (ZVD) - 3 impulses, ±50% robustness
    ZVD,
    /// Extra Insensitive (EI) - 3 impulses, ±75% robustness
    EI,
}

/// Command buffer entry
#[derive(Clone, Copy, Debug)]
struct CommandEntry {
    time: I16F16,
    value: I16F16,
}

/// Input shaper configuration
#[derive(Clone, Copy, Debug)]
pub struct InputShaperConfig {
    /// Natural frequency (rad/s)
    pub omega_n: I16F16,
    /// Damping ratio (0-1)
    pub zeta: I16F16,
    /// Shaper type
    pub shaper_type: ShaperType,
}

impl Default for InputShaperConfig {
    fn default() -> Self {
        Self {
            omega_n: I16F16::from_num(15.0),   // 15 rad/s = 2.4 Hz
            zeta: I16F16::from_num(0.05),      // 5% damping
            shaper_type: ShaperType::ZVD,      // Most practical choice
        }
    }
}

/// Input shaper for vibration suppression
pub struct InputShaper {
    config: InputShaperConfig,

    /// Impulse sequence
    impulses: heapless::Vec<Impulse, MAX_IMPULSES>,

    /// Command history buffer for convolution
    buffer: Deque<CommandEntry, MAX_BUFFER_SIZE>,

    /// Damped natural frequency
    omega_d: I16F16,
}

impl InputShaper {
    /// Create new input shaper
    pub fn new(config: InputShaperConfig) -> Self {
        // Calculate damped natural frequency
        let zeta_sq = config.zeta * config.zeta;
        let omega_d = if config.zeta < I16F16::ONE {
            // omega_d = omega_n * sqrt(1 - zeta^2)
            let one_minus_zeta_sq = I16F16::ONE - zeta_sq;
            config.omega_n * Self::sqrt_approx(one_minus_zeta_sq)
        } else {
            I16F16::ZERO  // Overdamped
        };

        let mut shaper = Self {
            config,
            impulses: heapless::Vec::new(),
            buffer: Deque::new(),
            omega_d,
        };

        shaper.compute_impulses();
        shaper
    }

    /// Compute impulse sequence based on shaper type
    fn compute_impulses(&mut self) {
        self.impulses.clear();

        // Period (use damped frequency if available, otherwise undamped)
        let freq = if self.omega_d > I16F16::ZERO {
            self.omega_d
        } else {
            self.config.omega_n
        };

        let two_pi = I16F16::from_num(6.283185307179586);  // 2π
        let period = two_pi / freq;

        // Calculate K factor (damping term)
        let k = if self.config.zeta < I16F16::ONE {
            // K = exp(-zeta * pi / sqrt(1 - zeta^2))
            let pi = I16F16::from_num(3.141592653589793);
            let one_minus_zeta_sq = I16F16::ONE - self.config.zeta * self.config.zeta;
            let sqrt_term = Self::sqrt_approx(one_minus_zeta_sq);
            let exponent = -(self.config.zeta * pi) / sqrt_term;
            Self::exp_approx(exponent)
        } else {
            I16F16::ZERO
        };

        match self.config.shaper_type {
            ShaperType::ZV => {
                // ZV: 2 impulses
                let a1 = I16F16::ONE / (I16F16::ONE + k);
                let a2 = k / (I16F16::ONE + k);

                let _ = self.impulses.push(Impulse { time: I16F16::ZERO, amplitude: a1 });
                let _ = self.impulses.push(Impulse { time: period / I16F16::from_num(2.0), amplitude: a2 });
            }
            ShaperType::ZVD => {
                // ZVD: 3 impulses
                let two = I16F16::from_num(2.0);
                let k_sq = k * k;
                let denom = I16F16::ONE + two * k + k_sq;

                let a1 = I16F16::ONE / denom;
                let a2 = (two * k) / denom;
                let a3 = k_sq / denom;

                let _ = self.impulses.push(Impulse { time: I16F16::ZERO, amplitude: a1 });
                let _ = self.impulses.push(Impulse { time: period / two, amplitude: a2 });
                let _ = self.impulses.push(Impulse { time: period, amplitude: a3 });
            }
            ShaperType::EI => {
                // EI: 3 impulses (optimized for robustness)
                let two = I16F16::from_num(2.0);
                let a1 = I16F16::from_num(0.25);
                let a2 = I16F16::from_num(0.50);
                let a3 = I16F16::from_num(0.25);

                let _ = self.impulses.push(Impulse { time: I16F16::ZERO, amplitude: a1 });
                let _ = self.impulses.push(Impulse { time: period / two, amplitude: a2 });
                let _ = self.impulses.push(Impulse { time: period, amplitude: a3 });
            }
        }
    }

    /// Apply input shaping to command
    ///
    /// # Arguments
    /// * `command` - Raw command value
    /// * `time` - Current time (seconds)
    ///
    /// # Returns
    /// Shaped command value
    pub fn shape(&mut self, command: I16F16, time: I16F16) -> I16F16 {
        // Add to buffer
        let entry = CommandEntry { time, value: command };
        if self.buffer.is_full() {
            let _ = self.buffer.pop_front();
        }
        let _ = self.buffer.push_back(entry);

        // Convolve with impulses
        let mut shaped_output = I16F16::ZERO;

        for impulse in &self.impulses {
            let target_time = time - impulse.time;

            if target_time < I16F16::ZERO {
                continue;
            }

            // Find command at target_time using linear interpolation
            if let Some(cmd_value) = self.interpolate_command(target_time) {
                shaped_output += impulse.amplitude * cmd_value;
            }
        }

        shaped_output
    }

    /// Interpolate command value at given time
    fn interpolate_command(&self, target_time: I16F16) -> Option<I16F16> {
        if self.buffer.is_empty() {
            return None;
        }

        // Find bracketing points
        let mut idx = 0;
        for (i, entry) in self.buffer.iter().enumerate() {
            if entry.time <= target_time {
                idx = i;
            } else {
                break;
            }
        }

        // Get value at idx
        let entry0 = self.buffer.get(idx)?;

        // Try to interpolate with next point
        if let Some(entry1) = self.buffer.get(idx + 1) {
            if entry1.time > entry0.time {
                // Linear interpolation
                let alpha = (target_time - entry0.time) / (entry1.time - entry0.time);
                let value = entry0.value + alpha * (entry1.value - entry0.value);
                Some(value)
            } else {
                Some(entry0.value)
            }
        } else {
            Some(entry0.value)
        }
    }

    /// Get time delay introduced by shaper (seconds)
    pub fn get_delay(&self) -> I16F16 {
        self.impulses
            .iter()
            .map(|imp| imp.time)
            .max()
            .unwrap_or(I16F16::ZERO)
    }

    /// Reset shaper state
    pub fn reset(&mut self) {
        self.buffer.clear();
    }

    /// Update configuration
    pub fn set_config(&mut self, config: InputShaperConfig) {
        self.config = config;

        // Recalculate omega_d
        let zeta_sq = config.zeta * config.zeta;
        self.omega_d = if config.zeta < I16F16::ONE {
            let one_minus_zeta_sq = I16F16::ONE - zeta_sq;
            config.omega_n * Self::sqrt_approx(one_minus_zeta_sq)
        } else {
            I16F16::ZERO
        };

        self.compute_impulses();
        self.reset();
    }

    /// Get current configuration
    pub fn config(&self) -> InputShaperConfig {
        self.config
    }

    /// Get impulse sequence (for inspection/debugging)
    pub fn impulses(&self) -> &[Impulse] {
        &self.impulses
    }

    // ========================================================================
    // Math approximations for embedded systems
    // ========================================================================

    /// Square root approximation using Newton-Raphson
    fn sqrt_approx(x: I16F16) -> I16F16 {
        if x <= I16F16::ZERO {
            return I16F16::ZERO;
        }

        if x == I16F16::ONE {
            return I16F16::ONE;
        }

        // Initial guess
        let mut estimate = x / I16F16::from_num(2.0);

        // Newton-Raphson iterations
        for _ in 0..5 {
            estimate = (estimate + x / estimate) / I16F16::from_num(2.0);
        }

        estimate
    }

    /// Exponential approximation for small negative values
    /// exp(x) for x in [-2, 0]
    fn exp_approx(x: I16F16) -> I16F16 {
        if x >= I16F16::ZERO {
            return I16F16::ONE;
        }

        if x < I16F16::from_num(-5.0) {
            return I16F16::ZERO;  // Very small
        }

        // Padé approximation: exp(x) ≈ (2 + x) / (2 - x)
        // Valid for small x
        let two = I16F16::from_num(2.0);
        let num = two + x;
        let denom = two - x;

        if denom > I16F16::ZERO {
            num / denom
        } else {
            I16F16::ZERO
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zv_shaper_creation() {
        let config = InputShaperConfig {
            omega_n: I16F16::from_num(15.0),
            zeta: I16F16::from_num(0.05),
            shaper_type: ShaperType::ZV,
        };

        let shaper = InputShaper::new(config);

        // ZV should have 2 impulses
        assert_eq!(shaper.impulses.len(), 2);

        // Amplitudes should sum to 1
        let sum: I16F16 = shaper.impulses.iter().map(|imp| imp.amplitude).sum();
        assert!((sum - I16F16::ONE).abs() < I16F16::from_num(0.01));
    }

    #[test]
    fn test_zvd_shaper_creation() {
        let config = InputShaperConfig {
            omega_n: I16F16::from_num(15.0),
            zeta: I16F16::from_num(0.05),
            shaper_type: ShaperType::ZVD,
        };

        let shaper = InputShaper::new(config);

        // ZVD should have 3 impulses
        assert_eq!(shaper.impulses.len(), 3);

        // Amplitudes should sum to 1
        let sum: I16F16 = shaper.impulses.iter().map(|imp| imp.amplitude).sum();
        assert!((sum - I16F16::ONE).abs() < I16F16::from_num(0.01));
    }

    #[test]
    fn test_shaping_step_command() {
        let config = InputShaperConfig {
            omega_n: I16F16::from_num(15.0),
            zeta: I16F16::from_num(0.05),
            shaper_type: ShaperType::ZV,
        };

        let mut shaper = InputShaper::new(config);

        // Apply step command
        let dt = I16F16::from_num(0.001);  // 1 ms
        let duration = I16F16::from_num(1.0);

        let mut time = I16F16::ZERO;
        let mut shaped_value = I16F16::ZERO;

        while time < duration {
            let command = I16F16::ONE;  // Step input
            shaped_value = shaper.shape(command, time);
            time += dt;
        }

        // After sufficient time, shaped output should approach 1.0
        assert!(shaped_value > I16F16::from_num(0.9));
        assert!(shaped_value <= I16F16::from_num(1.1));
    }

    #[test]
    fn test_get_delay() {
        let config = InputShaperConfig {
            omega_n: I16F16::from_num(15.0),
            zeta: I16F16::from_num(0.05),
            shaper_type: ShaperType::ZV,
        };

        let shaper = InputShaper::new(config);
        let delay = shaper.get_delay();

        // ZV delay should be approximately half period
        let period = I16F16::from_num(2.0) * I16F16::from_num(3.14159) / config.omega_n;
        let expected_delay = period / I16F16::from_num(2.0);

        assert!((delay - expected_delay).abs() < I16F16::from_num(0.05));
    }

    #[test]
    fn test_reset() {
        let config = InputShaperConfig::default();
        let mut shaper = InputShaper::new(config);

        // Add some commands
        for i in 0..10 {
            let time = I16F16::from_num(i as f32 * 0.001);
            let _ = shaper.shape(I16F16::ONE, time);
        }

        assert!(!shaper.buffer.is_empty());

        // Reset
        shaper.reset();
        assert!(shaper.buffer.is_empty());
    }

    #[test]
    fn test_sqrt_approx() {
        let x = I16F16::from_num(4.0);
        let sqrt_x = InputShaper::sqrt_approx(x);
        let expected = I16F16::from_num(2.0);

        assert!((sqrt_x - expected).abs() < I16F16::from_num(0.01));
    }

    #[test]
    fn test_exp_approx() {
        let x = I16F16::from_num(-1.0);
        let exp_x = InputShaper::exp_approx(x);

        // exp(-1) ≈ 0.368
        let expected = I16F16::from_num(0.368);
        assert!((exp_x - expected).abs() < I16F16::from_num(0.1));
    }
}
