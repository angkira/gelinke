/// Adaptive control module
///
/// Implements load-adaptive features inspired by TMC5160T:
/// - coolStep: Adaptive current reduction (50-75% power savings)
/// - dcStep: Load-adaptive velocity derating (stall prevention)
/// - stallGuard: Sensorless stall detection
///
/// Performance targets:
/// - Load estimation: < 10 µs
/// - coolStep update: < 20 µs
/// - dcStep update: < 10 µs
/// - stallGuard check: < 5 µs

use fixed::types::I16F16;

/// Ring buffer for averaging current samples
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

/// Load estimator configuration
#[derive(Clone, Copy, Debug)]
pub struct LoadEstimatorConfig {
    /// Rated current (A) - motor specification
    pub rated_current: I16F16,
    /// Rated torque (Nm) - motor specification
    pub rated_torque: I16F16,
    /// Torque constant (Nm/A) - k_t
    pub torque_constant: I16F16,
    /// Stall current threshold (A) - for stall prediction
    pub stall_current_threshold: I16F16,
}

impl Default for LoadEstimatorConfig {
    fn default() -> Self {
        Self {
            rated_current: I16F16::from_num(3.0),        // 3A typical for small joint
            rated_torque: I16F16::from_num(0.3),          // 0.3 Nm
            torque_constant: I16F16::from_num(0.1),       // 0.1 Nm/A
            stall_current_threshold: I16F16::from_num(2.5), // 83% of rated
        }
    }
}

/// Load estimation from current measurements
///
/// Estimates load percentage based on Q-axis (torque-producing) current.
pub struct LoadEstimator {
    /// Moving average of Q-axis current
    current_history: RingBuffer<I16F16, 50>,
    /// Configuration
    config: LoadEstimatorConfig,
    /// Last estimated load percentage (0-100%)
    last_load: f32,
}

impl LoadEstimator {
    /// Create new load estimator
    pub fn new(config: LoadEstimatorConfig) -> Self {
        Self {
            current_history: RingBuffer::new(),
            config,
            last_load: 0.0,
        }
    }

    /// Update with current measurement
    ///
    /// Call this at FOC loop rate (10 kHz).
    /// Performance: < 5 µs
    #[inline]
    pub fn update(&mut self, current_q: I16F16) {
        self.current_history.push(current_q.abs());
    }

    /// Estimate current load percentage (0-100%)
    ///
    /// Performance: < 10 µs
    pub fn estimate_load(&mut self) -> f32 {
        let avg_current = self.current_history.average();
        
        // Load = (current / rated_current) * 100
        let load = if self.config.rated_current > I16F16::ZERO {
            (avg_current / self.config.rated_current).to_num::<f32>() * 100.0
        } else {
            0.0
        };

        // Clamp to reasonable range
        self.last_load = load.clamp(0.0, 150.0); // Allow overload up to 150%
        self.last_load
    }

    /// Get last estimated load without recalculation
    #[inline]
    pub fn last_load(&self) -> f32 {
        self.last_load
    }

    /// Predict if stall is imminent
    ///
    /// Returns true if average current exceeds threshold.
    pub fn predict_stall(&self) -> bool {
        let avg_current = self.current_history.average();
        avg_current >= self.config.stall_current_threshold
    }

    /// Calculate torque estimate from current
    pub fn calculate_torque(&self, current_q: I16F16) -> I16F16 {
        current_q * self.config.torque_constant
    }

    /// Get configuration
    pub fn config(&self) -> LoadEstimatorConfig {
        self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: LoadEstimatorConfig) {
        self.config = config;
    }
}

impl Default for LoadEstimator {
    fn default() -> Self {
        Self::new(LoadEstimatorConfig::default())
    }
}

/// coolStep configuration
#[derive(Clone, Copy, Debug)]
pub struct CoolStepConfig {
    /// Minimum current percentage (don't go below this for safety)
    pub min_current_percent: f32,
    /// Maximum current scale (normally 1.0 = 100%)
    pub max_current_scale: f32,
    /// Adaptation rate (how fast to respond to load changes)
    pub adaptation_rate: f32,
    /// Load threshold below which to start reducing current
    pub reduction_threshold: f32,
    /// Maximum change per update cycle (for stability)
    pub max_change_per_cycle: f32,
}

impl Default for CoolStepConfig {
    fn default() -> Self {
        Self {
            min_current_percent: 0.3,      // Never below 30%
            max_current_scale: 1.0,         // 100% maximum
            adaptation_rate: 0.1,           // 10% per update
            reduction_threshold: 30.0,      // Start reducing below 30% load
            max_change_per_cycle: 0.05,     // Max 5% change per cycle
        }
    }
}

/// coolStep: Adaptive current reduction
///
/// Reduces motor current when load is low, saving power.
/// Inspired by TMC5160T coolStep feature.
pub struct CoolStep {
    /// Load estimator
    load_estimator: LoadEstimator,
    /// Configuration
    config: CoolStepConfig,
    /// Current scale factor (0.0 - 1.0)
    current_scale: f32,
    /// Enabled state
    enabled: bool,
    /// Total power savings (Watt-hours)
    total_savings_wh: f32,
}

impl CoolStep {
    /// Create new coolStep controller
    pub fn new(load_config: LoadEstimatorConfig, config: CoolStepConfig) -> Self {
        Self {
            load_estimator: LoadEstimator::new(load_config),
            config,
            current_scale: 1.0, // Start at full current
            enabled: false,
            total_savings_wh: 0.0,
        }
    }

    /// Enable coolStep
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable coolStep
    pub fn disable(&mut self) {
        self.enabled = false;
        self.current_scale = 1.0; // Reset to full current
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Update load estimation
    ///
    /// Call at FOC loop rate (10 kHz).
    /// Performance: < 5 µs
    #[inline]
    pub fn update_load(&mut self, current_q: I16F16) {
        self.load_estimator.update(current_q);
    }

    /// Calculate and apply current scaling
    ///
    /// Returns current scale factor (0.0 - 1.0).
    /// Performance: < 20 µs
    pub fn calculate_scale(&mut self) -> f32 {
        if !self.enabled {
            return 1.0;
        }

        let load = self.load_estimator.estimate_load();

        // Calculate target scale based on load
        let target_scale = if load < self.config.reduction_threshold {
            // Low load: reduce current
            // Linear interpolation from min to max based on load
            let ratio = load / self.config.reduction_threshold;
            self.config.min_current_percent + 
                (self.config.max_current_scale - self.config.min_current_percent) * ratio
        } else {
            // High load: full current
            self.config.max_current_scale
        };

        // Rate limit changes (prevent instability)
        let change = target_scale - self.current_scale;
        let limited_change = change.clamp(
            -self.config.max_change_per_cycle,
            self.config.max_change_per_cycle
        );
        
        self.current_scale = (self.current_scale + limited_change)
            .clamp(self.config.min_current_percent, self.config.max_current_scale);

        self.current_scale
    }

    /// Get current scale factor
    #[inline]
    pub fn current_scale(&self) -> f32 {
        self.current_scale
    }

    /// Get load estimator
    pub fn load_estimator(&self) -> &LoadEstimator {
        &self.load_estimator
    }

    /// Get power savings percentage
    ///
    /// Returns percentage of power saved (0-100%).
    pub fn get_savings(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        (1.0 - self.current_scale) * 100.0
    }

    /// Update total energy savings
    ///
    /// Call periodically with time delta.
    pub fn update_energy_savings(&mut self, dt_s: f32, power_watts: f32) {
        if self.enabled {
            let saved_power = power_watts * (1.0 - self.current_scale);
            self.total_savings_wh += saved_power * dt_s / 3600.0; // Convert to Wh
        }
    }

    /// Get total energy savings (Watt-hours)
    pub fn total_savings_wh(&self) -> f32 {
        self.total_savings_wh
    }

    /// Reset energy savings counter
    pub fn reset_savings(&mut self) {
        self.total_savings_wh = 0.0;
    }

    /// Get configuration
    pub fn config(&self) -> CoolStepConfig {
        self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: CoolStepConfig) {
        self.config = config;
    }
}

impl Default for CoolStep {
    fn default() -> Self {
        Self::new(LoadEstimatorConfig::default(), CoolStepConfig::default())
    }
}

/// dcStep configuration
#[derive(Clone, Copy, Debug)]
pub struct DcStepConfig {
    /// Load threshold to start derating (%)
    pub load_threshold: f32,
    /// Maximum velocity derating (0.0 - 1.0, e.g., 0.2 = 20% reduction)
    pub max_derating: f32,
    /// Critical load threshold for minimum velocity (%)
    pub critical_threshold: f32,
    /// Minimum velocity scale (safety limit)
    pub min_velocity_scale: f32,
}

impl Default for DcStepConfig {
    fn default() -> Self {
        Self {
            load_threshold: 70.0,       // Start derating at 70% load
            max_derating: 0.2,           // Max 20% velocity reduction
            critical_threshold: 90.0,    // Critical at 90% load
            min_velocity_scale: 0.8,     // Never below 80% velocity
        }
    }
}

/// dcStep: Load-adaptive velocity derating
///
/// Reduces maximum velocity under high load to prevent stalls.
/// Inspired by TMC5160T dcStep feature.
pub struct DcStep {
    /// Configuration
    config: DcStepConfig,
    /// Current velocity scale factor (0.0 - 1.0)
    velocity_scale: f32,
    /// Enabled state
    enabled: bool,
}

impl DcStep {
    /// Create new dcStep controller
    pub fn new(config: DcStepConfig) -> Self {
        Self {
            config,
            velocity_scale: 1.0, // Start at full velocity
            enabled: false,
        }
    }

    /// Enable dcStep
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable dcStep
    pub fn disable(&mut self) {
        self.enabled = false;
        self.velocity_scale = 1.0; // Reset to full velocity
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Update velocity scaling based on load
    ///
    /// Returns velocity scale factor (0.0 - 1.0).
    /// Performance: < 10 µs
    pub fn update(&mut self, load: f32) -> f32 {
        if !self.enabled {
            return 1.0;
        }

        self.velocity_scale = if load < self.config.load_threshold {
            // Normal load: full velocity
            1.0
        } else if load < self.config.critical_threshold {
            // High load: linear derating
            let excess_load = load - self.config.load_threshold;
            let load_range = self.config.critical_threshold - self.config.load_threshold;
            let derating_factor = excess_load / load_range;
            
            1.0 - (self.config.max_derating * derating_factor)
        } else {
            // Critical load: minimum velocity
            self.config.min_velocity_scale
        };

        // Ensure within bounds
        self.velocity_scale = self.velocity_scale.clamp(
            self.config.min_velocity_scale,
            1.0
        );

        self.velocity_scale
    }

    /// Get current velocity scale
    #[inline]
    pub fn velocity_scale(&self) -> f32 {
        self.velocity_scale
    }

    /// Check if derating is active
    pub fn is_derating(&self) -> bool {
        self.enabled && self.velocity_scale < 1.0
    }

    /// Get derating percentage
    pub fn derating_percent(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }
        (1.0 - self.velocity_scale) * 100.0
    }

    /// Get configuration
    pub fn config(&self) -> DcStepConfig {
        self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: DcStepConfig) {
        self.config = config;
    }
}

impl Default for DcStep {
    fn default() -> Self {
        Self::new(DcStepConfig::default())
    }
}

/// Stall status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StallStatus {
    /// Normal operation
    Normal,
    /// Warning: high load, might stall
    Warning,
    /// Stalled: motor cannot move
    Stalled,
}

/// stallGuard configuration
#[derive(Clone, Copy, Debug)]
pub struct StallGuardConfig {
    /// Current threshold for stall detection (A)
    pub current_threshold: I16F16,
    /// Velocity threshold for stall detection (rad/s)
    pub velocity_threshold: I16F16,
    /// Duration threshold for stall confirmation (ms)
    pub duration_threshold_ms: u32,
    /// Warning threshold (current percentage for warning)
    pub warning_threshold_percent: f32,
}

impl Default for StallGuardConfig {
    fn default() -> Self {
        Self {
            current_threshold: I16F16::from_num(2.5),    // 2.5A
            velocity_threshold: I16F16::from_num(0.05),  // 0.05 rad/s (~3 deg/s)
            duration_threshold_ms: 100,                   // 100ms
            warning_threshold_percent: 70.0,              // Warning at 70% load
        }
    }
}

/// stallGuard: Sensorless stall detection
///
/// Detects motor stall by monitoring current and velocity.
/// Inspired by TMC5160T stallGuard feature.
pub struct StallGuard {
    /// Configuration
    config: StallGuardConfig,
    /// Stall counter (incremented when conditions met)
    stall_counter_ms: u32,
    /// Current status
    status: StallStatus,
    /// Enabled state
    enabled: bool,
    /// Last update time (for debouncing)
    last_update_time_us: u64,
}

impl StallGuard {
    /// Create new stallGuard detector
    pub fn new(config: StallGuardConfig) -> Self {
        Self {
            config,
            stall_counter_ms: 0,
            status: StallStatus::Normal,
            enabled: false,
            last_update_time_us: 0,
        }
    }

    /// Enable stallGuard
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// Disable stallGuard
    pub fn disable(&mut self) {
        self.enabled = false;
        self.status = StallStatus::Normal;
        self.stall_counter_ms = 0;
    }

    /// Check if enabled
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Update stall detection
    ///
    /// Call periodically with current sensor data.
    /// Performance: < 5 µs
    pub fn update(
        &mut self,
        current_q: I16F16,
        velocity: I16F16,
        load_percent: f32,
        current_time_us: u64,
    ) -> StallStatus {
        if !self.enabled {
            return StallStatus::Normal;
        }

        // Calculate time delta
        let dt_ms = if self.last_update_time_us > 0 {
            (current_time_us - self.last_update_time_us) / 1000
        } else {
            0
        };
        self.last_update_time_us = current_time_us;

        // Check stall conditions
        let high_current = current_q.abs() >= self.config.current_threshold;
        let low_velocity = velocity.abs() <= self.config.velocity_threshold;

        if high_current && low_velocity {
            // Potential stall: increment counter
            self.stall_counter_ms = self.stall_counter_ms.saturating_add(dt_ms as u32);
            
            if self.stall_counter_ms >= self.config.duration_threshold_ms {
                self.status = StallStatus::Stalled;
            } else {
                self.status = StallStatus::Warning;
            }
        } else if load_percent >= self.config.warning_threshold_percent {
            // High load warning
            self.status = StallStatus::Warning;
            // Decay counter
            self.stall_counter_ms = self.stall_counter_ms.saturating_sub(dt_ms as u32);
        } else {
            // Normal operation
            self.status = StallStatus::Normal;
            self.stall_counter_ms = 0;
        }

        self.status
    }

    /// Check if currently stalled
    #[inline]
    pub fn is_stalled(&self) -> bool {
        self.status == StallStatus::Stalled
    }

    /// Check if warning
    #[inline]
    pub fn is_warning(&self) -> bool {
        self.status == StallStatus::Warning
    }

    /// Get current status
    #[inline]
    pub fn status(&self) -> StallStatus {
        self.status
    }

    /// Get stall detection confidence (0-100%)
    pub fn confidence(&self) -> f32 {
        if !self.enabled {
            return 0.0;
        }

        let ratio = self.stall_counter_ms as f32 / self.config.duration_threshold_ms as f32;
        (ratio * 100.0).min(100.0)
    }

    /// Reset stall detection
    pub fn reset(&mut self) {
        self.stall_counter_ms = 0;
        self.status = StallStatus::Normal;
    }

    /// Get configuration
    pub fn config(&self) -> StallGuardConfig {
        self.config
    }

    /// Update configuration
    pub fn set_config(&mut self, config: StallGuardConfig) {
        self.config = config;
    }
}

impl Default for StallGuard {
    fn default() -> Self {
        Self::new(StallGuardConfig::default())
    }
}

/// Adaptive control coordinator
///
/// Combines coolStep, dcStep, and stallGuard for intelligent motor control.
pub struct AdaptiveController {
    pub coolstep: CoolStep,
    pub dcstep: DcStep,
    pub stallguard: StallGuard,
}

impl AdaptiveController {
    /// Create new adaptive controller
    pub fn new(
        load_config: LoadEstimatorConfig,
        coolstep_config: CoolStepConfig,
        dcstep_config: DcStepConfig,
        stallguard_config: StallGuardConfig,
    ) -> Self {
        Self {
            coolstep: CoolStep::new(load_config, coolstep_config),
            dcstep: DcStep::new(dcstep_config),
            stallguard: StallGuard::new(stallguard_config),
        }
    }

    /// Update all adaptive features
    ///
    /// Call at FOC loop rate or lower (e.g., 1 kHz).
    /// Returns (current_scale, velocity_scale, stall_status).
    pub fn update(
        &mut self,
        current_q: I16F16,
        velocity: I16F16,
        current_time_us: u64,
    ) -> (f32, f32, StallStatus) {
        // Update coolStep load estimation
        self.coolstep.update_load(current_q);
        
        // Calculate current scale
        let current_scale = self.coolstep.calculate_scale();
        
        // Get load estimate for dcStep and stallGuard
        let load = self.coolstep.load_estimator().last_load();
        
        // Update dcStep velocity scaling
        let velocity_scale = self.dcstep.update(load);
        
        // Update stallGuard
        let stall_status = self.stallguard.update(
            current_q,
            velocity,
            load,
            current_time_us,
        );
        
        (current_scale, velocity_scale, stall_status)
    }

    /// Enable all adaptive features
    pub fn enable_all(&mut self) {
        self.coolstep.enable();
        self.dcstep.enable();
        self.stallguard.enable();
    }

    /// Disable all adaptive features
    pub fn disable_all(&mut self) {
        self.coolstep.disable();
        self.dcstep.disable();
        self.stallguard.disable();
    }

    /// Get current load estimate
    pub fn load(&self) -> f32 {
        self.coolstep.load_estimator().last_load()
    }
}

impl Default for AdaptiveController {
    fn default() -> Self {
        Self::new(
            LoadEstimatorConfig::default(),
            CoolStepConfig::default(),
            DcStepConfig::default(),
            StallGuardConfig::default(),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    #[test]
    fn test_load_estimator_creation() {
        let estimator = LoadEstimator::default();
        assert_eq!(estimator.last_load(), 0.0);
    }

    #[test]
    fn test_load_estimation() {
        let mut estimator = LoadEstimator::default();
        
        // Simulate 1A current (33% of 3A rated)
        let current = I16F16::from_num(1.0);
        for _ in 0..50 {
            estimator.update(current);
        }
        
        let load = estimator.estimate_load();
        assert!((load - 33.33).abs() < 1.0); // ~33%
    }

    #[test]
    fn test_coolstep_scaling() {
        let mut coolstep = CoolStep::default();
        coolstep.enable();
        
        // Low load: should reduce current
        let current = I16F16::from_num(0.3); // 10% load
        for _ in 0..50 {
            coolstep.update_load(current);
        }
        
        let scale = coolstep.calculate_scale();
        assert!(scale < 1.0); // Should reduce current
        assert!(scale >= coolstep.config().min_current_percent); // But not below min
    }

    #[test]
    fn test_coolstep_high_load() {
        let mut coolstep = CoolStep::default();
        coolstep.enable();
        
        // High load: should use full current
        let current = I16F16::from_num(2.5); // ~83% load
        for _ in 0..50 {
            coolstep.update_load(current);
        }
        
        let scale = coolstep.calculate_scale();
        assert!((scale - 1.0).abs() < 0.1); // Should be near 100%
    }

    #[test]
    fn test_dcstep_derating() {
        let mut dcstep = DcStep::default();
        dcstep.enable();
        
        // Normal load: full velocity
        let scale = dcstep.update(50.0);
        assert_eq!(scale, 1.0);
        
        // High load: derating
        let scale = dcstep.update(80.0);
        assert!(scale < 1.0);
        assert!(scale >= dcstep.config().min_velocity_scale);
        
        // Critical load: minimum velocity
        let scale = dcstep.update(95.0);
        assert_eq!(scale, dcstep.config().min_velocity_scale);
    }

    #[test]
    fn test_stallguard_detection() {
        let mut stallguard = StallGuard::default();
        stallguard.enable();
        
        let high_current = I16F16::from_num(2.6);
        let low_velocity = I16F16::from_num(0.01);
        
        // Should detect stall after threshold time
        for i in 0..150 {
            let time_us = (i * 1000) as u64; // 1ms steps
            let status = stallguard.update(high_current, low_velocity, 85.0, time_us);
            
            if i < 100 {
                assert_ne!(status, StallStatus::Stalled); // Not yet
            } else {
                assert_eq!(status, StallStatus::Stalled); // After 100ms
            }
        }
    }

    #[test]
    fn test_stallguard_normal_operation() {
        let mut stallguard = StallGuard::default();
        stallguard.enable();
        
        let normal_current = I16F16::from_num(1.0);
        let normal_velocity = I16F16::from_num(1.0);
        
        let status = stallguard.update(normal_current, normal_velocity, 30.0, 0);
        assert_eq!(status, StallStatus::Normal);
    }

    #[test]
    fn test_adaptive_controller_integration() {
        let mut controller = AdaptiveController::default();
        controller.enable_all();
        
        let current = I16F16::from_num(1.5);
        let velocity = I16F16::from_num(0.5);
        
        let (current_scale, velocity_scale, status) = controller.update(
            current,
            velocity,
            1_000_000,
        );
        
        assert!(current_scale > 0.0 && current_scale <= 1.0);
        assert!(velocity_scale > 0.0 && velocity_scale <= 1.0);
        assert_eq!(status, StallStatus::Normal);
    }

    #[test]
    fn test_ring_buffer_averaging() {
        let mut buffer: RingBuffer<I16F16, 5> = RingBuffer::new();
        
        buffer.push(I16F16::from_num(1.0));
        buffer.push(I16F16::from_num(2.0));
        buffer.push(I16F16::from_num(3.0));
        
        let avg = buffer.average();
        assert!((avg.to_num::<f32>() - 2.0).abs() < 0.01);
    }
}

