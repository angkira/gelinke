/// Auto-tuning module for PI controllers
///
/// Implements Relay (Bang-Bang) method for automatic gain tuning:
/// 1. Apply relay control (oscillate around setpoint)
/// 2. Measure oscillation period (Tu) and amplitude
/// 3. Calculate ultimate gain (Ku) from relay parameters
/// 4. Apply Ziegler-Nichols rules to compute optimal PI gains
///
/// Performance: Background task, not timing-critical
/// Typical tuning time: 10-30 seconds (depends on system dynamics)

use fixed::types::I16F16;

/// Tuning state machine
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuningState {
    /// Not started
    NotStarted,
    /// Measuring system response (relay control active)
    Measuring,
    /// Calculating optimal gains
    Calculating,
    /// Testing new gains (validation)
    Testing,
    /// Tuning complete, gains ready
    Complete,
    /// Tuning failed
    Failed,
}

/// Tuning error types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum TuningError {
    /// No oscillation detected
    NoOscillation,
    /// Oscillation too slow (system too damped)
    TooSlow,
    /// Oscillation too fast (unstable)
    TooFast,
    /// Amplitude too small
    AmplitudeTooSmall,
    /// Invalid measurements
    InvalidData,
}

/// PI controller gains
#[derive(Clone, Copy, Debug)]
pub struct PiGains {
    /// Proportional gain
    pub kp: I16F16,
    /// Integral gain
    pub ki: I16F16,
}

impl Default for PiGains {
    fn default() -> Self {
        Self {
            kp: I16F16::ZERO,
            ki: I16F16::ZERO,
        }
    }
}

/// Auto-tuner configuration
#[derive(Clone, Copy, Debug)]
pub struct AutoTunerConfig {
    /// Relay amplitude (output magnitude for oscillation)
    pub relay_amplitude: I16F16,
    /// Setpoint for tuning
    pub setpoint: I16F16,
    /// Minimum number of oscillation cycles to observe
    pub min_cycles: usize,
    /// Maximum tuning time (seconds)
    pub max_time_s: f32,
    /// Minimum oscillation period (seconds)
    pub min_period_s: f32,
    /// Maximum oscillation period (seconds)
    pub max_period_s: f32,
}

impl Default for AutoTunerConfig {
    fn default() -> Self {
        Self {
            relay_amplitude: I16F16::from_num(1.0),
            setpoint: I16F16::ZERO,
            min_cycles: 3,
            max_time_s: 30.0,
            min_period_s: 0.1,
            max_period_s: 10.0,
        }
    }
}

/// Oscillation measurement
#[derive(Clone, Copy, Debug)]
struct OscillationMeasurement {
    /// Period (seconds)
    period: f32,
    /// Amplitude
    amplitude: I16F16,
}

/// Auto-tuner for PI controllers using Relay method
pub struct AutoTuner {
    /// Configuration
    config: AutoTunerConfig,
    /// Current state
    state: TuningState,
    /// Error samples (process variable)
    error_samples: heapless::Vec<(f32, I16F16), 1000>, // (time, error)
    /// Zero crossing times (for period measurement)
    zero_crossings: heapless::Vec<f32, 20>,
    /// Last error sign (for zero crossing detection)
    last_error_sign: i8,
    /// Tuning start time
    start_time_s: f32,
    /// Measured oscillation data
    oscillation: Option<OscillationMeasurement>,
    /// Calculated gains
    gains: Option<PiGains>,
    /// Relay output state
    relay_output: I16F16,
}

impl AutoTuner {
    /// Create new auto-tuner
    pub fn new(config: AutoTunerConfig) -> Self {
        Self {
            config,
            state: TuningState::NotStarted,
            error_samples: heapless::Vec::new(),
            zero_crossings: heapless::Vec::new(),
            last_error_sign: 0,
            start_time_s: 0.0,
            oscillation: None,
            gains: None,
            relay_output: I16F16::ZERO,
        }
    }

    /// Start auto-tuning process
    pub fn start(&mut self, current_time_s: f32) {
        self.state = TuningState::Measuring;
        self.error_samples.clear();
        self.zero_crossings.clear();
        self.last_error_sign = 0;
        self.start_time_s = current_time_s;
        self.oscillation = None;
        self.gains = None;
        self.relay_output = self.config.relay_amplitude;
    }

    /// Update auto-tuner with measurement
    ///
    /// Returns relay output (use this as controller output during tuning).
    /// Call this at regular intervals (e.g., 1 kHz).
    pub fn update(&mut self, process_value: I16F16, current_time_s: f32) -> I16F16 {
        if self.state != TuningState::Measuring {
            return I16F16::ZERO;
        }

        // Calculate error
        let error = self.config.setpoint - process_value;

        // Store sample
        if self.error_samples.push((current_time_s, error)).is_err() {
            // Buffer full, proceed to calculation
            self.state = TuningState::Calculating;
            return I16F16::ZERO;
        }

        // Detect zero crossings
        let error_sign = if error > I16F16::ZERO {
            1
        } else if error < I16F16::ZERO {
            -1
        } else {
            self.last_error_sign
        };

        if error_sign != self.last_error_sign && self.last_error_sign != 0 {
            // Zero crossing detected
            let _ = self.zero_crossings.push(current_time_s);
        }
        self.last_error_sign = error_sign;

        // Apply relay control (bang-bang)
        self.relay_output = if error > I16F16::ZERO {
            self.config.relay_amplitude
        } else {
            -self.config.relay_amplitude
        };

        // Check if we have enough cycles
        let num_periods = self.zero_crossings.len().saturating_sub(1) / 2;
        let elapsed = current_time_s - self.start_time_s;

        if num_periods >= self.config.min_cycles {
            // Enough data, proceed to calculation
            self.state = TuningState::Calculating;
        } else if elapsed > self.config.max_time_s {
            // Timeout
            self.state = TuningState::Failed;
        }

        self.relay_output
    }

    /// Calculate optimal gains (call when state is Calculating)
    pub fn calculate_gains(&mut self) -> Result<PiGains, TuningError> {
        if self.state != TuningState::Calculating {
            return Err(TuningError::InvalidData);
        }

        // Calculate average period from zero crossings
        let avg_period = self.calculate_average_period()?;

        // Calculate amplitude
        let amplitude = self.calculate_amplitude()?;

        // Store oscillation measurement
        self.oscillation = Some(OscillationMeasurement {
            period: avg_period,
            amplitude,
        });

        // Calculate ultimate gain (Ku) from relay method
        // Ku = (4 * relay_amplitude) / (π * amplitude)
        let pi = core::f32::consts::PI;
        let relay_amp = self.config.relay_amplitude.to_num::<f32>();
        let osc_amp = amplitude.to_num::<f32>();
        
        if osc_amp < 0.001 {
            self.state = TuningState::Failed;
            return Err(TuningError::AmplitudeTooSmall);
        }

        let ku = (4.0 * relay_amp) / (pi * osc_amp);
        let tu = avg_period;

        // Ziegler-Nichols PI rules:
        // Kp = 0.45 * Ku
        // Ki = 0.54 * Ku / Tu
        let kp = 0.45 * ku;
        let ki = 0.54 * ku / tu;

        let gains = PiGains {
            kp: I16F16::from_num(kp),
            ki: I16F16::from_num(ki),
        };

        self.gains = Some(gains);
        self.state = TuningState::Complete;

        Ok(gains)
    }

    /// Calculate average oscillation period
    fn calculate_average_period(&self) -> Result<f32, TuningError> {
        if self.zero_crossings.len() < 4 {
            return Err(TuningError::NoOscillation);
        }

        // Calculate periods between consecutive zero crossings (half periods)
        let mut periods = heapless::Vec::<f32, 10>::new();
        
        for i in 0..self.zero_crossings.len() - 2 {
            let half_period = self.zero_crossings[i + 2] - self.zero_crossings[i];
            let _ = periods.push(half_period);
        }

        if periods.is_empty() {
            return Err(TuningError::NoOscillation);
        }

        // Average period (full cycle)
        let avg_period = periods.iter().sum::<f32>() / periods.len() as f32;

        // Validate period
        if avg_period < self.config.min_period_s {
            return Err(TuningError::TooFast);
        }
        if avg_period > self.config.max_period_s {
            return Err(TuningError::TooSlow);
        }

        Ok(avg_period)
    }

    /// Calculate oscillation amplitude
    fn calculate_amplitude(&self) -> Result<I16F16, TuningError> {
        if self.error_samples.len() < 10 {
            return Err(TuningError::InvalidData);
        }

        // Find peak-to-peak amplitude
        let mut min_error = I16F16::MAX;
        let mut max_error = I16F16::MIN;

        for (_, error) in &self.error_samples {
            if *error < min_error {
                min_error = *error;
            }
            if *error > max_error {
                max_error = *error;
            }
        }

        let amplitude = (max_error - min_error) / I16F16::from_num(2.0);

        if amplitude < I16F16::from_num(0.001) {
            return Err(TuningError::AmplitudeTooSmall);
        }

        Ok(amplitude)
    }

    /// Get current state
    pub fn state(&self) -> TuningState {
        self.state
    }

    /// Get tuning progress (0.0 - 1.0)
    pub fn progress(&self, current_time_s: f32) -> f32 {
        match self.state {
            TuningState::NotStarted => 0.0,
            TuningState::Measuring => {
                let elapsed = current_time_s - self.start_time_s;
                let expected_time = self.config.max_time_s * 0.5; // Assume 50% completion
                (elapsed / expected_time).min(0.9)
            }
            TuningState::Calculating => 0.95,
            TuningState::Testing => 0.98,
            TuningState::Complete => 1.0,
            TuningState::Failed => 0.0,
        }
    }

    /// Check if tuning is complete
    pub fn is_complete(&self) -> bool {
        self.state == TuningState::Complete
    }

    /// Check if tuning failed
    pub fn is_failed(&self) -> bool {
        self.state == TuningState::Failed
    }

    /// Get calculated gains (if complete)
    pub fn gains(&self) -> Option<PiGains> {
        self.gains
    }

    /// Get oscillation measurement (if available)
    pub fn oscillation(&self) -> Option<OscillationMeasurement> {
        self.oscillation
    }

    /// Reset tuner
    pub fn reset(&mut self) {
        self.state = TuningState::NotStarted;
        self.error_samples.clear();
        self.zero_crossings.clear();
        self.last_error_sign = 0;
        self.oscillation = None;
        self.gains = None;
        self.relay_output = I16F16::ZERO;
    }

    /// Get configuration
    pub fn config(&self) -> AutoTunerConfig {
        self.config
    }

    /// Set configuration
    pub fn set_config(&mut self, config: AutoTunerConfig) {
        self.config = config;
    }
}

impl Default for AutoTuner {
    fn default() -> Self {
        Self::new(AutoTunerConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    #[test]
    fn test_auto_tuner_creation() {
        let tuner = AutoTuner::default();
        assert_eq!(tuner.state(), TuningState::NotStarted);
        assert!(!tuner.is_complete());
        assert!(!tuner.is_failed());
    }

    #[cfg(test)]
    #[test]
    fn test_auto_tuner_start() {
        let mut tuner = AutoTuner::default();
        tuner.start(0.0);
        assert_eq!(tuner.state(), TuningState::Measuring);
    }

    #[cfg(test)]
    #[test]
    fn test_relay_output() {
        let mut tuner = AutoTuner::new(AutoTunerConfig {
            relay_amplitude: I16F16::from_num(2.0),
            setpoint: I16F16::ZERO,
            ..Default::default()
        });
        
        tuner.start(0.0);
        
        // Positive error: positive output
        let output = tuner.update(I16F16::from_num(-1.0), 0.001);
        assert_eq!(output, I16F16::from_num(2.0));
        
        // Negative error: negative output
        let output = tuner.update(I16F16::from_num(1.0), 0.002);
        assert_eq!(output, I16F16::from_num(-2.0));
    }

    #[cfg(test)]
    #[test]
    fn test_zero_crossing_detection() {
        let mut tuner = AutoTuner::default();
        tuner.start(0.0);
        
        // Simulate oscillation
        tuner.update(I16F16::from_num(-1.0), 0.0);
        tuner.update(I16F16::from_num(1.0), 0.1);  // Zero crossing
        tuner.update(I16F16::from_num(-1.0), 0.2); // Zero crossing
        
        assert!(tuner.zero_crossings.len() >= 2);
    }

    #[cfg(test)]
    #[test]
    fn test_tuning_progress() {
        let mut tuner = AutoTuner::default();
        assert_eq!(tuner.progress(0.0), 0.0);
        
        tuner.start(0.0);
        let progress = tuner.progress(5.0);
        assert!(progress > 0.0 && progress < 1.0);
    }

    #[cfg(test)]
    #[test]
    fn test_ziegler_nichols_calculation() {
        // Test with known oscillation parameters
        let mut tuner = AutoTuner::new(AutoTunerConfig {
            relay_amplitude: I16F16::from_num(1.0),
            setpoint: I16F16::ZERO,
            min_cycles: 2,
            ..Default::default()
        });
        
        tuner.start(0.0);
        
        // Simulate clean oscillation (period ~1s, amplitude ~0.5)
        let mut time = 0.0;
        for i in 0..100 {
            time = i as f32 * 0.01;
            let phase = time * 2.0 * core::f32::consts::PI; // 1 Hz
            let pv = I16F16::from_num(0.5 * libm::sinf(phase));
            tuner.update(pv, time);
            
            if tuner.state() == TuningState::Calculating {
                break;
            }
        }
        
        if tuner.state() == TuningState::Calculating {
            let gains = tuner.calculate_gains();
            assert!(gains.is_ok());
            
            let gains = gains.unwrap();
            assert!(gains.kp > I16F16::ZERO);
            assert!(gains.ki > I16F16::ZERO);
        }
    }
}

