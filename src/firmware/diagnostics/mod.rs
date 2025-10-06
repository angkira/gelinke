/// Diagnostics module for health monitoring and predictive maintenance
///
/// Provides:
/// - Real-time health scoring
/// - Trend analysis for temperature, current, errors
/// - Predictive failure detection
/// - Time-to-failure estimation

pub mod health;

pub use health::{HealthMonitor, HealthScore, HealthWarning, HealthThresholds};

