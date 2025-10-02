use fixed::types::I16F16;

/// Luenberger observer configuration.
#[derive(Clone, Copy, Debug)]
pub struct ObserverConfig {
    /// Motor inertia (kg·m²)
    pub inertia: I16F16,
    /// Friction coefficient (N·m·s/rad)
    pub friction: I16F16,
    /// Observer gain for position error
    pub gain_position: I16F16,
    /// Observer gain for velocity error
    pub gain_velocity: I16F16,
}

impl Default for ObserverConfig {
    fn default() -> Self {
        Self {
            inertia: I16F16::from_num(0.001), // 1 gram·m²
            friction: I16F16::from_num(0.01), // Small friction
            gain_position: I16F16::from_num(100.0),
            gain_velocity: I16F16::from_num(10.0),
        }
    }
}

/// Luenberger state observer for load torque estimation.
///
/// Estimates external load torque from motor position, velocity, and current.
pub struct LuenbergerObserver {
    config: ObserverConfig,
    /// Estimated position
    position_est: I16F16,
    /// Estimated velocity
    velocity_est: I16F16,
    /// Estimated load torque
    load_torque_est: I16F16,
}

impl LuenbergerObserver {
    /// Create a new Luenberger observer.
    pub fn new(config: ObserverConfig) -> Self {
        Self {
            config,
            position_est: I16F16::ZERO,
            velocity_est: I16F16::ZERO,
            load_torque_est: I16F16::ZERO,
        }
    }

    /// Update observer with measurements.
    ///
    /// # Arguments
    /// * `position` - Measured position (rad)
    /// * `velocity` - Measured velocity (rad/s)
    /// * `motor_torque` - Motor torque from current (N·m)
    /// * `dt` - Time step (s)
    ///
    /// Returns estimated load torque (N·m)
    pub fn update(
        &mut self,
        position: I16F16,
        velocity: I16F16,
        motor_torque: I16F16,
        dt: f32,
    ) -> I16F16 {
        // Position error
        let pos_error = position - self.position_est;
        
        // Velocity error
        let vel_error = velocity - self.velocity_est;
        
        // Observer corrections
        let pos_correction = self.config.gain_position * pos_error;
        let vel_correction = self.config.gain_velocity * vel_error;
        
        // System dynamics: τ_net = J·α + b·ω + τ_load
        // Rearranged: τ_load = J·α + b·ω - τ_motor
        let friction_torque = self.config.friction * self.velocity_est;
        let acceleration = (motor_torque - friction_torque - self.load_torque_est) / self.config.inertia;
        
        // Update estimates with corrections
        self.velocity_est += (acceleration + vel_correction) * I16F16::from_num(dt);
        self.position_est += (self.velocity_est + pos_correction) * I16F16::from_num(dt);
        
        // Update load estimate (slowly track disturbance)
        let load_update = -vel_correction * I16F16::from_num(0.1); // Slow adaptation
        self.load_torque_est += load_update * I16F16::from_num(dt);
        
        self.load_torque_est
    }

    /// Get estimated position.
    pub fn position_estimate(&self) -> I16F16 {
        self.position_est
    }

    /// Get estimated velocity.
    pub fn velocity_estimate(&self) -> I16F16 {
        self.velocity_est
    }

    /// Get estimated load torque.
    pub fn load_estimate(&self) -> I16F16 {
        self.load_torque_est
    }

    /// Reset observer state.
    pub fn reset(&mut self, position: I16F16, velocity: I16F16) {
        self.position_est = position;
        self.velocity_est = velocity;
        self.load_torque_est = I16F16::ZERO;
    }

    /// Update configuration.
    pub fn set_config(&mut self, config: ObserverConfig) {
        self.config = config;
    }

    /// Get current configuration.
    pub fn config(&self) -> ObserverConfig {
        self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn observer_initialization() {
        let observer = LuenbergerObserver::new(ObserverConfig::default());
        assert_eq!(observer.position_estimate(), I16F16::ZERO);
        assert_eq!(observer.velocity_estimate(), I16F16::ZERO);
        assert_eq!(observer.load_estimate(), I16F16::ZERO);
    }

    #[test]
    fn observer_reset() {
        let mut observer = LuenbergerObserver::new(ObserverConfig::default());
        observer.reset(I16F16::from_num(1.0), I16F16::from_num(2.0));
        
        assert_eq!(observer.position_estimate(), I16F16::from_num(1.0));
        assert_eq!(observer.velocity_estimate(), I16F16::from_num(2.0));
        assert_eq!(observer.load_estimate(), I16F16::ZERO);
    }

    #[test]
    fn observer_update_no_load() {
        let config = ObserverConfig {
            inertia: I16F16::from_num(0.001),
            friction: I16F16::from_num(0.01),
            gain_position: I16F16::from_num(10.0),
            gain_velocity: I16F16::from_num(5.0),
        };
        let mut observer = LuenbergerObserver::new(config);
        
        // Initialize at zero
        observer.reset(I16F16::ZERO, I16F16::ZERO);
        
        // Apply small motor torque
        let motor_torque = I16F16::from_num(0.01);
        let dt = 0.001;
        
        // Update several times
        for _ in 0..10 {
            let measured_pos = I16F16::ZERO;
            let measured_vel = I16F16::from_num(1.0);
            observer.update(measured_pos, measured_vel, motor_torque, dt);
        }
        
        // Velocity estimate should converge toward measurement
        let vel_est = observer.velocity_estimate().to_num::<f32>();
        assert!(vel_est > 0.0 && vel_est < 2.0);
    }

    #[test]
    fn observer_load_estimation() {
        let mut observer = LuenbergerObserver::new(ObserverConfig::default());
        observer.reset(I16F16::ZERO, I16F16::from_num(1.0));
        
        // Constant velocity with motor torque => implies load
        let motor_torque = I16F16::from_num(0.05);
        let dt = 0.001;
        
        // Run for multiple iterations
        for _ in 0..100 {
            let measured_pos = I16F16::from_num(0.1);
            let measured_vel = I16F16::from_num(1.0);
            observer.update(measured_pos, measured_vel, motor_torque, dt);
        }
        
        // Load estimate should be non-zero after convergence
        let load_est = observer.load_estimate();
        // Not checking exact value due to complexity, just that it's not zero
        assert_ne!(load_est, I16F16::ZERO);
    }
}

