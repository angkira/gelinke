/// Health monitoring and scoring module
///
/// Tracks system health metrics and provides:
/// - Overall health score (0-100%)
/// - Active warnings
/// - Trend analysis
/// - Predictive failure detection

use fixed::types::I16F16;

/// Ring buffer for trend analysis
#[derive(Clone, Copy, Debug)]
struct TrendBuffer<const N: usize> {
    data: [(u64, f32); N],
    index: usize,
    count: usize,
}

impl<const N: usize> TrendBuffer<N> {
    fn new() -> Self {
        Self {
            data: [(0, 0.0); N],
            index: 0,
            count: 0,
        }
    }

    fn push(&mut self, time_us: u64, value: f32) {
        self.data[self.index] = (time_us, value);
        self.index = (self.index + 1) % N;
        if self.count < N {
            self.count += 1;
        }
    }

    /// Calculate linear regression slope (rate of change per second)
    fn slope(&self) -> f32 {
        if self.count < 2 {
            return 0.0;
        }

        let n = self.count as f32;
        let mut sum_t = 0.0;
        let mut sum_v = 0.0;
        let mut sum_tv = 0.0;
        let mut sum_t2 = 0.0;

        // Use first timestamp as reference
        let t0 = self.data[0].0 as f32;

        for i in 0..self.count {
            let (time, value) = self.data[i];
            let t = (time as f32 - t0) / 1_000_000.0; // Convert to seconds
            sum_t += t;
            sum_v += value;
            sum_tv += t * value;
            sum_t2 += t * t;
        }

        let denominator = n * sum_t2 - sum_t * sum_t;
        if denominator.abs() < 0.001 {
            return 0.0;
        }

        (n * sum_tv - sum_t * sum_v) / denominator
    }

    /// Predict value at time ahead (seconds from now)
    fn predict(&self, time_ahead_s: f32) -> Option<f32> {
        if self.count == 0 {
            return None;
        }

        let slope = self.slope();
        let last_value = self.data[(self.index + N - 1) % N].1;
        
        Some(last_value + slope * time_ahead_s)
    }

    /// Get average value
    fn average(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }

        let sum: f32 = self.data[..self.count].iter().map(|(_, v)| v).sum();
        sum / self.count as f32
    }

    /// Get last value
    fn last(&self) -> f32 {
        if self.count == 0 {
            return 0.0;
        }
        self.data[(self.index + N - 1) % N].1
    }

    /// Check if trend is concerning (above threshold)
    fn is_concerning(&self, threshold: f32) -> bool {
        self.slope().abs() > threshold
    }
}

/// Health warning types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum HealthWarning {
    /// Temperature trending upward
    TemperatureTrend,
    /// High temperature threshold exceeded
    HighTemperature,
    /// Current trending upward (wear indication)
    CurrentTrend,
    /// High current usage
    HighCurrent,
    /// Error rate increasing
    FrequentErrors,
    /// Tracking performance degrading
    PerformanceDegradation,
    /// Predicted failure imminent
    FailureImminent,
}

/// Health thresholds configuration
#[derive(Clone, Copy, Debug)]
pub struct HealthThresholds {
    // Temperature thresholds
    pub temp_warning_c: f32,
    pub temp_critical_c: f32,
    pub temp_trend_threshold: f32, // °C/minute

    // Current thresholds
    pub current_warning_a: f32,
    pub current_critical_a: f32,
    pub current_trend_threshold: f32, // A/minute

    // Error rate thresholds
    pub error_rate_warning: f32, // errors/minute
    pub error_rate_critical: f32,

    // Performance thresholds
    pub tracking_error_warning_deg: f32,
    pub tracking_error_critical_deg: f32,
}

impl Default for HealthThresholds {
    fn default() -> Self {
        Self {
            temp_warning_c: 60.0,
            temp_critical_c: 80.0,
            temp_trend_threshold: 5.0, // 5°C/min is concerning

            current_warning_a: 2.5,
            current_critical_a: 4.0,
            current_trend_threshold: 0.5, // 0.5 A/min increase

            error_rate_warning: 10.0, // 10 errors/min
            error_rate_critical: 30.0,

            tracking_error_warning_deg: 5.0,
            tracking_error_critical_deg: 10.0,
        }
    }
}

/// Health score (0-100%)
#[derive(Clone, Copy, Debug)]
pub struct HealthScore {
    /// Overall health (0-100%)
    pub overall: f32,
    /// Temperature component
    pub temperature: f32,
    /// Current component
    pub current: f32,
    /// Error rate component
    pub errors: f32,
    /// Performance component
    pub performance: f32,
}

impl Default for HealthScore {
    fn default() -> Self {
        Self {
            overall: 100.0,
            temperature: 100.0,
            current: 100.0,
            errors: 100.0,
            performance: 100.0,
        }
    }
}

/// Health monitor for predictive maintenance
pub struct HealthMonitor {
    /// Configuration thresholds
    thresholds: HealthThresholds,

    /// Temperature trend (last 100 samples)
    temperature_trend: TrendBuffer<100>,

    /// Current trend (last 100 samples)
    current_trend: TrendBuffer<100>,

    /// Error count history (last 100 events)
    error_history: TrendBuffer<100>,

    /// Tracking error history
    tracking_error_trend: TrendBuffer<50>,

    /// Last calculated health score
    health_score: HealthScore,

    /// Active warnings
    active_warnings: heapless::Vec<HealthWarning, 8>,

    /// Total error count
    total_errors: u32,

    /// Last update time
    last_update_time_us: u64,
}

impl HealthMonitor {
    /// Create new health monitor
    pub fn new(thresholds: HealthThresholds) -> Self {
        Self {
            thresholds,
            temperature_trend: TrendBuffer::new(),
            current_trend: TrendBuffer::new(),
            error_history: TrendBuffer::new(),
            tracking_error_trend: TrendBuffer::new(),
            health_score: HealthScore::default(),
            active_warnings: heapless::Vec::new(),
            total_errors: 0,
            last_update_time_us: 0,
        }
    }

    /// Update with current measurements
    pub fn update(
        &mut self,
        current_time_us: u64,
        temperature_c: f32,
        current_q_a: f32,
        tracking_error_deg: f32,
    ) {
        // Update trends
        self.temperature_trend.push(current_time_us, temperature_c);
        self.current_trend.push(current_time_us, current_q_a.abs());
        self.tracking_error_trend.push(current_time_us, tracking_error_deg.abs());

        // Calculate health score
        self.health_score = self.calculate_health_score();

        // Update warnings
        self.update_warnings();

        self.last_update_time_us = current_time_us;
    }

    /// Log an error event
    pub fn log_error(&mut self, current_time_us: u64) {
        self.total_errors += 1;
        self.error_history.push(current_time_us, 1.0);
    }

    /// Calculate overall health score
    fn calculate_health_score(&self) -> HealthScore {
        let temp_score = self.temperature_health_score();
        let current_score = self.current_health_score();
        let error_score = self.error_health_score();
        let perf_score = self.performance_health_score();

        // Weighted average (all equal weight)
        let overall = (temp_score + current_score + error_score + perf_score) / 4.0;

        HealthScore {
            overall: overall.clamp(0.0, 100.0),
            temperature: temp_score,
            current: current_score,
            errors: error_score,
            performance: perf_score,
        }
    }

    /// Calculate temperature health component
    fn temperature_health_score(&self) -> f32 {
        let temp = self.temperature_trend.last();
        
        if temp >= self.thresholds.temp_critical_c {
            0.0 // Critical
        } else if temp >= self.thresholds.temp_warning_c {
            // Linear between warning and critical
            let range = self.thresholds.temp_critical_c - self.thresholds.temp_warning_c;
            let excess = temp - self.thresholds.temp_warning_c;
            50.0 * (1.0 - excess / range)
        } else {
            // Good range
            100.0
        }
    }

    /// Calculate current health component
    fn current_health_score(&self) -> f32 {
        let current = self.current_trend.average();
        
        if current >= self.thresholds.current_critical_a {
            0.0
        } else if current >= self.thresholds.current_warning_a {
            let range = self.thresholds.current_critical_a - self.thresholds.current_warning_a;
            let excess = current - self.thresholds.current_warning_a;
            50.0 * (1.0 - excess / range)
        } else {
            100.0
        }
    }

    /// Calculate error rate health component
    fn error_health_score(&self) -> f32 {
        let error_rate = self.calculate_error_rate();
        
        if error_rate >= self.thresholds.error_rate_critical {
            0.0
        } else if error_rate >= self.thresholds.error_rate_warning {
            let range = self.thresholds.error_rate_critical - self.thresholds.error_rate_warning;
            let excess = error_rate - self.thresholds.error_rate_warning;
            50.0 * (1.0 - excess / range)
        } else {
            100.0
        }
    }

    /// Calculate performance health component
    fn performance_health_score(&self) -> f32 {
        let error = self.tracking_error_trend.average();
        
        if error >= self.thresholds.tracking_error_critical_deg {
            0.0
        } else if error >= self.thresholds.tracking_error_warning_deg {
            let range = self.thresholds.tracking_error_critical_deg - self.thresholds.tracking_error_warning_deg;
            let excess = error - self.thresholds.tracking_error_warning_deg;
            50.0 * (1.0 - excess / range)
        } else {
            100.0
        }
    }

    /// Calculate error rate (errors per minute)
    fn calculate_error_rate(&self) -> f32 {
        // Count errors in last minute
        let current_time_us = self.last_update_time_us;
        let one_minute_ago = current_time_us.saturating_sub(60_000_000);

        let mut error_count = 0;
        for i in 0..self.error_history.count {
            let (time, _) = self.error_history.data[i];
            if time >= one_minute_ago {
                error_count += 1;
            }
        }

        error_count as f32
    }

    /// Update active warnings list
    fn update_warnings(&mut self) {
        self.active_warnings.clear();

        // Temperature warnings
        let temp = self.temperature_trend.last();
        if temp >= self.thresholds.temp_warning_c {
            let _ = self.active_warnings.push(HealthWarning::HighTemperature);
        }
        if self.temperature_trend.is_concerning(self.thresholds.temp_trend_threshold / 60.0) {
            let _ = self.active_warnings.push(HealthWarning::TemperatureTrend);
        }

        // Current warnings
        let current = self.current_trend.average();
        if current >= self.thresholds.current_warning_a {
            let _ = self.active_warnings.push(HealthWarning::HighCurrent);
        }
        if self.current_trend.is_concerning(self.thresholds.current_trend_threshold / 60.0) {
            let _ = self.active_warnings.push(HealthWarning::CurrentTrend);
        }

        // Error rate warnings
        let error_rate = self.calculate_error_rate();
        if error_rate >= self.thresholds.error_rate_warning {
            let _ = self.active_warnings.push(HealthWarning::FrequentErrors);
        }

        // Performance warnings
        let tracking_error = self.tracking_error_trend.average();
        if tracking_error >= self.thresholds.tracking_error_warning_deg {
            let _ = self.active_warnings.push(HealthWarning::PerformanceDegradation);
        }

        // Failure prediction
        if self.health_score.overall < 20.0 {
            let _ = self.active_warnings.push(HealthWarning::FailureImminent);
        }
    }

    /// Predict time to failure (hours)
    ///
    /// Returns Some(hours) if failure is predicted, None if system is healthy.
    pub fn time_to_failure(&self) -> Option<f32> {
        // Check temperature trend
        if let Some(temp_future) = self.temperature_trend.predict(3600.0) {
            if temp_future >= self.thresholds.temp_critical_c {
                // Calculate when temperature will hit critical
                let temp_now = self.temperature_trend.last();
                let slope = self.temperature_trend.slope() * 60.0; // per minute
                
                if slope > 0.1 {
                    let time_to_critical = (self.thresholds.temp_critical_c - temp_now) / slope;
                    return Some(time_to_critical / 60.0); // Convert to hours
                }
            }
        }

        // Check current trend
        if let Some(current_future) = self.current_trend.predict(3600.0) {
            if current_future >= self.thresholds.current_critical_a {
                let current_now = self.current_trend.average();
                let slope = self.current_trend.slope() * 60.0;
                
                if slope > 0.01 {
                    let time_to_critical = (self.thresholds.current_critical_a - current_now) / slope;
                    return Some(time_to_critical / 60.0);
                }
            }
        }

        None
    }

    /// Get current health score
    pub fn health_score(&self) -> HealthScore {
        self.health_score
    }

    /// Get active warnings
    pub fn warnings(&self) -> &[HealthWarning] {
        &self.active_warnings
    }

    /// Get total error count
    pub fn total_errors(&self) -> u32 {
        self.total_errors
    }

    /// Get configuration
    pub fn thresholds(&self) -> HealthThresholds {
        self.thresholds
    }

    /// Update configuration
    pub fn set_thresholds(&mut self, thresholds: HealthThresholds) {
        self.thresholds = thresholds;
    }
}

impl Default for HealthMonitor {
    fn default() -> Self {
        Self::new(HealthThresholds::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    #[test]
    fn test_health_monitor_creation() {
        let monitor = HealthMonitor::default();
        assert_eq!(monitor.health_score().overall, 100.0);
        assert_eq!(monitor.warnings().len(), 0);
    }

    #[cfg(test)]
    #[test]
    fn test_normal_operation() {
        let mut monitor = HealthMonitor::default();
        
        for i in 0..10 {
            monitor.update(i * 1_000_000, 25.0, 1.0, 0.5);
        }
        
        let score = monitor.health_score();
        assert!(score.overall > 90.0); // Should be healthy
        assert_eq!(monitor.warnings().len(), 0);
    }

    #[cfg(test)]
    #[test]
    fn test_high_temperature_warning() {
        let mut monitor = HealthMonitor::default();
        
        monitor.update(0, 65.0, 1.0, 0.5); // Above warning threshold
        
        let score = monitor.health_score();
        assert!(score.overall < 100.0);
        assert!(monitor.warnings().contains(&HealthWarning::HighTemperature));
    }

    #[cfg(test)]
    #[test]
    fn test_temperature_trend_detection() {
        let mut monitor = HealthMonitor::default();
        
        // Simulate rising temperature
        for i in 0..20 {
            let temp = 25.0 + (i as f32 * 2.0); // +2°C per sample
            monitor.update(i * 1_000_000, temp, 1.0, 0.5);
        }
        
        // Should detect trend
        assert!(monitor.temperature_trend.slope() > 0.0);
    }

    #[cfg(test)]
    #[test]
    fn test_error_logging() {
        let mut monitor = HealthMonitor::default();
        
        monitor.log_error(0);
        monitor.log_error(1_000_000);
        monitor.log_error(2_000_000);
        
        assert_eq!(monitor.total_errors(), 3);
    }

    #[cfg(test)]
    #[test]
    fn test_trend_buffer_slope() {
        let mut buffer: TrendBuffer<10> = TrendBuffer::new();
        
        // Linear increase
        for i in 0..10 {
            buffer.push(i * 1_000_000, i as f32);
        }
        
        let slope = buffer.slope();
        assert!((slope - 1.0).abs() < 0.1); // Should be ~1.0 per second
    }

    #[cfg(test)]
    #[test]
    fn test_time_to_failure_prediction() {
        let mut monitor = HealthMonitor::default();
        
        // Simulate rapidly rising temperature
        for i in 0..50 {
            let temp = 50.0 + (i as f32 * 0.5); // +0.5°C per sample
            monitor.update(i * 1_000_000, temp, 1.0, 0.5);
        }
        
        let ttf = monitor.time_to_failure();
        // Should predict failure (temperature will exceed critical)
        assert!(ttf.is_some());
    }
}

