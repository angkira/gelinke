/// Telemetry collection and streaming module
///
/// Collects data in FOC loop with minimal overhead (< 5 µs target)
/// and provides telemetry streaming at configurable rates.

use fixed::types::I16F16;
use irpc::protocol::{TelemetryStream, TelemetryMode};
use heapless::Vec;

/// Ring buffer for averaging samples
#[derive(Clone, Copy, Debug)]
struct RingBuffer<T, const N: usize> {
    data: [T; N],
    index: usize,
    count: usize,
}

impl<T: Copy + Default, const N: usize> RingBuffer<T, N> {
    fn new() -> Self {
        Self {
            data: [T::default(); N],
            index: 0,
            count: 0,
        }
    }

    fn push(&mut self, value: T) {
        self.data[self.index] = value;
        self.index = (self.index + 1) % N;
        if self.count < N {
            self.count += 1;
        }
    }
}

// Specialized implementation for I16F16 with any size
impl<const N: usize> RingBuffer<I16F16, N> {
    fn average(&self) -> I16F16 {
        if self.count == 0 {
            return I16F16::ZERO;
        }
        
        let sum: I16F16 = self.data[..self.count]
            .iter()
            .fold(I16F16::ZERO, |acc, &x| acc + x);
        
        sum / I16F16::from_num(self.count)
    }
}

/// Telemetry configuration
#[derive(Clone, Copy, Debug)]
pub struct TelemetryConfig {
    pub mode: TelemetryMode,
    pub rate_hz: u16,
    pub change_threshold: f32,
}

impl Default for TelemetryConfig {
    fn default() -> Self {
        Self {
            mode: TelemetryMode::OnDemand,
            rate_hz: 100,              // 100 Hz default for Periodic
            change_threshold: 1.0,      // 1 degree/unit for OnChange
        }
    }
}

/// Sample data collected in FOC loop
#[derive(Clone, Copy, Debug, Default)]
pub struct TelemetrySample {
    pub position: I16F16,
    pub velocity: I16F16,
    pub current_d: I16F16,
    pub current_q: I16F16,
    pub voltage_d: I16F16,
    pub voltage_q: I16F16,
}

/// Telemetry collector - runs in FOC loop
pub struct TelemetryCollector {
    // Ring buffers for averaging (reduce noise)
    position_samples: RingBuffer<I16F16, 10>,
    velocity_samples: RingBuffer<I16F16, 10>,
    current_d_samples: RingBuffer<I16F16, 10>,
    current_q_samples: RingBuffer<I16F16, 10>,
    
    // Previous values for derivative calculation
    prev_velocity: I16F16,
    prev_time_us: u64,
    
    // Last telemetry values (for OnChange mode)
    last_position: f32,
    last_velocity: f32,
    
    // Timing
    last_send_time_us: u64,
    foc_loop_time_us: u16,
    
    // Configuration
    config: TelemetryConfig,
    
    // Status
    trajectory_active: bool,
    motion_active: bool,  // For adaptive mode
}

impl TelemetryCollector {
    /// Create new telemetry collector
    pub fn new() -> Self {
        Self {
            position_samples: RingBuffer::new(),
            velocity_samples: RingBuffer::new(),
            current_d_samples: RingBuffer::new(),
            current_q_samples: RingBuffer::new(),
            prev_velocity: I16F16::ZERO,
            prev_time_us: 0,
            last_position: 0.0,
            last_velocity: 0.0,
            last_send_time_us: 0,
            foc_loop_time_us: 0,
            config: TelemetryConfig::default(),
            trajectory_active: false,
            motion_active: false,
        }
    }

    /// Collect sample in FOC loop (< 5 µs target)
    ///
    /// This is called at 10 kHz and must be extremely fast.
    /// Only ring buffer updates and simple operations allowed.
    #[inline]
    pub fn collect_sample(&mut self, sample: TelemetrySample, current_time_us: u64) {
        // Update ring buffers (< 1 µs each)
        self.position_samples.push(sample.position);
        self.velocity_samples.push(sample.velocity);
        self.current_d_samples.push(sample.current_d);
        self.current_q_samples.push(sample.current_q);
        
        // Update motion detection for adaptive mode
        let velocity_threshold = I16F16::from_num(0.1); // 0.1 rad/s
        self.motion_active = sample.velocity.abs() > velocity_threshold;
        
        // Store previous for derivatives
        self.prev_velocity = sample.velocity;
        self.prev_time_us = current_time_us;
    }

    /// Set FOC loop timing
    #[inline]
    pub fn set_foc_loop_time(&mut self, time_us: u16) {
        self.foc_loop_time_us = time_us;
    }

    /// Set trajectory active status
    #[inline]
    pub fn set_trajectory_active(&mut self, active: bool) {
        self.trajectory_active = active;
    }

    /// Configure telemetry streaming
    pub fn configure(&mut self, config: TelemetryConfig) {
        self.config = config;
        self.last_send_time_us = 0; // Reset timer
        defmt::info!("Telemetry configured: mode={:?}, rate={} Hz", 
            config.mode, config.rate_hz);
    }

    /// Check if telemetry should be sent now
    pub fn should_send(&self, current_time_us: u64) -> bool {
        match self.config.mode {
            TelemetryMode::OnDemand => {
                // Only send on explicit request (handled separately)
                false
            }
            TelemetryMode::Periodic => {
                if self.config.rate_hz == 0 {
                    return false;
                }
                let interval_us = 1_000_000 / self.config.rate_hz as u64;
                current_time_us - self.last_send_time_us >= interval_us
            }
            TelemetryMode::Streaming => {
                // Maximum rate: 1 kHz = every 1 ms
                current_time_us - self.last_send_time_us >= 1_000
            }
            TelemetryMode::OnChange => {
                // Check if values changed significantly
                let pos = self.position_samples.average().to_num::<f32>();
                let vel = self.velocity_samples.average().to_num::<f32>();
                
                let pos_changed = (pos - self.last_position).abs() > self.config.change_threshold;
                let vel_changed = (vel - self.last_velocity).abs() > self.config.change_threshold;
                
                pos_changed || vel_changed
            }
            TelemetryMode::Adaptive => {
                // Fast rate during motion, slow when idle
                let interval_us = if self.motion_active {
                    1_000       // 1 kHz during motion
                } else {
                    10_000      // 100 Hz when idle
                };
                current_time_us - self.last_send_time_us >= interval_us
            }
        }
    }

    /// Generate telemetry message
    ///
    /// Called when should_send() returns true.
    /// Can take more time (< 50 µs) since not in FOC loop.
    pub fn generate_telemetry(
        &mut self,
        current_time_us: u64,
        temperature_c: f32,
        warnings: u16,
    ) -> TelemetryStream {
        // Average ring buffer samples
        let position = self.position_samples.average();
        let velocity = self.velocity_samples.average();
        let current_d = self.current_d_samples.average();
        let current_q = self.current_q_samples.average();
        
        // Calculate acceleration (dv/dt)
        let dt_us = current_time_us.saturating_sub(self.prev_time_us);
        let dt_s = I16F16::from_num(dt_us as f32 / 1_000_000.0);
        let acceleration = if dt_s > I16F16::ZERO {
            (velocity - self.prev_velocity) / dt_s
        } else {
            I16F16::ZERO
        };
        
        // Convert to degrees (from radians)
        let rad_to_deg = 180.0 / core::f32::consts::PI;
        let position_deg = position.to_num::<f32>() * rad_to_deg;
        let velocity_deg = velocity.to_num::<f32>() * rad_to_deg;
        let acceleration_deg = acceleration.to_num::<f32>() * rad_to_deg;
        
        // FOC state (already in SI units)
        let current_d_a = current_d.to_num::<f32>();
        let current_q_a = current_q.to_num::<f32>();
        
        // Derived metrics
        let torque_estimate = self.calculate_torque(current_q_a);
        let power = self.calculate_power(current_d_a, current_q_a);
        let load_percent = self.calculate_load(current_q_a);
        
        // Update last sent values
        self.last_position = position_deg;
        self.last_velocity = velocity_deg;
        self.last_send_time_us = current_time_us;
        
        TelemetryStream {
            timestamp_us: current_time_us,
            position: position_deg,
            velocity: velocity_deg,
            acceleration: acceleration_deg,
            current_d: current_d_a,
            current_q: current_q_a,
            voltage_d: 0.0,  // TODO: collect from FOC
            voltage_q: 0.0,  // TODO: collect from FOC
            torque_estimate,
            power,
            load_percent,
            foc_loop_time_us: self.foc_loop_time_us,
            temperature_c,
            warnings,
            trajectory_active: self.trajectory_active,
        }
    }

    /// Force telemetry generation (for OnDemand mode)
    pub fn request_telemetry(
        &mut self,
        current_time_us: u64,
        temperature_c: f32,
        warnings: u16,
    ) -> TelemetryStream {
        self.generate_telemetry(current_time_us, temperature_c, warnings)
    }

    /// Calculate estimated torque from Q-axis current
    ///
    /// Simplified: τ = k_t * I_q
    /// where k_t is torque constant (motor-specific)
    fn calculate_torque(&self, current_q: f32) -> f32 {
        // TODO: Use actual motor parameters
        const K_T: f32 = 0.1; // Nm/A (typical for small BLDC)
        K_T * current_q
    }

    /// Calculate electrical power
    ///
    /// P = V_d * I_d + V_q * I_q
    fn calculate_power(&self, current_d: f32, current_q: f32) -> f32 {
        // TODO: Use actual voltages from FOC
        // Rough estimate: P ≈ R * (I_d² + I_q²)
        const R_PHASE: f32 = 1.0; // Ohms
        R_PHASE * (current_d * current_d + current_q * current_q)
    }

    /// Calculate load percentage
    ///
    /// Based on current vs maximum rated current
    fn calculate_load(&self, current_q: f32) -> f32 {
        const MAX_CURRENT: f32 = 10.0; // 10A rated (motor-specific)
        (current_q.abs() / MAX_CURRENT * 100.0).min(100.0)
    }

    /// Get current configuration
    pub fn config(&self) -> TelemetryConfig {
        self.config
    }
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ring_buffer_push_and_average() {
        let mut buffer: RingBuffer<I16F16, 5> = RingBuffer::new();
        
        buffer.push(I16F16::from_num(1.0));
        buffer.push(I16F16::from_num(2.0));
        buffer.push(I16F16::from_num(3.0));
        
        let avg = buffer.average();
        assert!((avg.to_num::<f32>() - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_telemetry_collector_creation() {
        let collector = TelemetryCollector::new();
        assert_eq!(collector.config.mode, TelemetryMode::OnDemand);
    }

    #[test]
    fn test_should_send_streaming_mode() {
        let mut collector = TelemetryCollector::new();
        collector.configure(TelemetryConfig {
            mode: TelemetryMode::Streaming,
            rate_hz: 1000,
            change_threshold: 1.0,
        });
        
        // Should send after 1 ms
        assert!(!collector.should_send(0));
        assert!(!collector.should_send(500));
        assert!(collector.should_send(1_000));
    }

    #[test]
    fn test_should_send_periodic_mode() {
        let mut collector = TelemetryCollector::new();
        collector.configure(TelemetryConfig {
            mode: TelemetryMode::Periodic,
            rate_hz: 100,  // 100 Hz = every 10 ms
            change_threshold: 1.0,
        });
        
        assert!(!collector.should_send(5_000));
        assert!(collector.should_send(10_000));
    }

    #[test]
    fn test_adaptive_mode_fast_during_motion() {
        let mut collector = TelemetryCollector::new();
        collector.configure(TelemetryConfig {
            mode: TelemetryMode::Adaptive,
            rate_hz: 0,
            change_threshold: 1.0,
        });
        
        // Collect sample with motion
        let sample = TelemetrySample {
            position: I16F16::ZERO,
            velocity: I16F16::from_num(1.0), // Moving
            current_d: I16F16::ZERO,
            current_q: I16F16::ZERO,
            voltage_d: I16F16::ZERO,
            voltage_q: I16F16::ZERO,
        };
        
        collector.collect_sample(sample, 0);
        
        // Should use fast rate (1 ms)
        assert!(collector.should_send(1_000));
    }

    #[test]
    fn test_torque_calculation() {
        let collector = TelemetryCollector::new();
        let torque = collector.calculate_torque(5.0);
        assert!((torque - 0.5).abs() < 0.01); // 0.1 * 5.0 = 0.5
    }
}

