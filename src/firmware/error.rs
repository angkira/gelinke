/// Firmware-wide error handling.
///
/// Provides a unified error type for all firmware operations,
/// enabling proper error propagation and recovery.

/// Firmware error types.
#[derive(Debug, Clone, Copy, defmt::Format, PartialEq)]
pub enum FirmwareError {
    // === Initialization Errors ===
    /// UART initialization failed (non-critical - can operate without logging).
    UartInitFailed,

    /// ADC initialization failed (CRITICAL - cannot operate without current sensing).
    AdcInitFailed,

    /// PWM initialization failed (CRITICAL - cannot operate without motor control).
    PwmInitFailed,

    /// CAN initialization failed (non-critical - can operate in Step-Dir mode).
    CanInitFailed,

    /// Flash storage initialization failed.
    FlashInitFailed,

    /// Encoder initialization failed (CRITICAL - cannot operate without position feedback).
    EncoderInitFailed,

    /// Motor driver initialization failed (CRITICAL).
    MotorDriverInitFailed,

    /// Watchdog initialization failed.
    WatchdogInitFailed,

    // === Runtime Errors ===
    /// Sensor read error (ADC, encoder, etc.).
    SensorReadError,

    /// Motor driver fault detected.
    MotorDriverFault,

    /// Calibration failed or invalid.
    CalibrationFailed,

    /// Communication timeout (CAN, UART, USB).
    CommunicationTimeout,

    /// Storage operation failed (flash read/write/erase).
    StorageError,

    /// Overcurrent condition detected.
    Overcurrent,

    /// Overvoltage condition detected.
    Overvoltage,

    /// Undervoltage condition detected.
    Undervoltage,

    /// Overtemperature condition detected.
    Overtemperature,

    // === Configuration Errors ===
    /// Invalid parameter value.
    InvalidParameter,

    /// Value out of acceptable range.
    OutOfRange,

    /// Configuration not set or invalid.
    InvalidConfiguration,

    // === Control Errors ===
    /// Control loop error (FOC, position, velocity).
    ControlError,

    /// Position/velocity limit exceeded.
    LimitExceeded,

    /// State machine in invalid state for operation.
    InvalidState,
}

/// Error severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ErrorSeverity {
    /// Informational only - no action required.
    Info,

    /// Warning - operation continues in degraded mode.
    Warning,

    /// Error - operation failed but system recoverable.
    Error,

    /// Critical - system cannot continue safely.
    Critical,
}

impl FirmwareError {
    /// Check if error is recoverable.
    pub const fn is_recoverable(&self) -> bool {
        match self {
            // Can continue without these (degraded mode)
            Self::UartInitFailed => true,
            Self::CanInitFailed => true,
            Self::CommunicationTimeout => true,
            Self::StorageError => true,

            // Cannot continue without these
            Self::AdcInitFailed => false,
            Self::PwmInitFailed => false,
            Self::EncoderInitFailed => false,
            Self::MotorDriverInitFailed => false,
            Self::MotorDriverFault => false,

            // Runtime faults may be recoverable
            Self::Overcurrent => true,       // Can throttle
            Self::Overtemperature => true,   // Can throttle
            Self::Overvoltage => false,       // Must stop
            Self::Undervoltage => false,      // Must stop

            // Control errors depend on context
            Self::ControlError => true,
            Self::LimitExceeded => true,
            Self::InvalidState => true,

            // Configuration errors
            Self::InvalidParameter => true,
            Self::OutOfRange => true,
            Self::InvalidConfiguration => false,

            // Other
            Self::SensorReadError => true,
            Self::CalibrationFailed => true,
            Self::FlashInitFailed => true,
            Self::WatchdogInitFailed => true,
        }
    }

    /// Get error severity.
    pub const fn severity(&self) -> ErrorSeverity {
        match self {
            // Critical - cannot operate
            Self::AdcInitFailed => ErrorSeverity::Critical,
            Self::PwmInitFailed => ErrorSeverity::Critical,
            Self::EncoderInitFailed => ErrorSeverity::Critical,
            Self::MotorDriverInitFailed => ErrorSeverity::Critical,
            Self::MotorDriverFault => ErrorSeverity::Critical,
            Self::Overvoltage => ErrorSeverity::Critical,
            Self::Undervoltage => ErrorSeverity::Critical,
            Self::InvalidConfiguration => ErrorSeverity::Critical,

            // Error - operation failed
            Self::Overcurrent => ErrorSeverity::Error,
            Self::Overtemperature => ErrorSeverity::Error,
            Self::ControlError => ErrorSeverity::Error,
            Self::LimitExceeded => ErrorSeverity::Error,
            Self::InvalidState => ErrorSeverity::Error,
            Self::SensorReadError => ErrorSeverity::Error,
            Self::CalibrationFailed => ErrorSeverity::Error,
            Self::StorageError => ErrorSeverity::Error,

            // Warning - degraded mode
            Self::UartInitFailed => ErrorSeverity::Warning,
            Self::CanInitFailed => ErrorSeverity::Warning,
            Self::CommunicationTimeout => ErrorSeverity::Warning,
            Self::FlashInitFailed => ErrorSeverity::Warning,
            Self::WatchdogInitFailed => ErrorSeverity::Warning,

            // Info - informational only
            Self::InvalidParameter => ErrorSeverity::Info,
            Self::OutOfRange => ErrorSeverity::Info,
        }
    }

    /// Check if error requires immediate motor stop.
    pub const fn requires_motor_stop(&self) -> bool {
        match self {
            Self::MotorDriverFault => true,
            Self::Overcurrent => true,
            Self::Overvoltage => true,
            Self::Undervoltage => true,
            Self::Overtemperature => true,
            Self::LimitExceeded => true,
            _ => false,
        }
    }

    /// Get human-readable error description.
    pub const fn description(&self) -> &'static str {
        match self {
            Self::UartInitFailed => "UART initialization failed",
            Self::AdcInitFailed => "ADC initialization failed",
            Self::PwmInitFailed => "PWM initialization failed",
            Self::CanInitFailed => "CAN initialization failed",
            Self::FlashInitFailed => "Flash storage initialization failed",
            Self::EncoderInitFailed => "Encoder initialization failed",
            Self::MotorDriverInitFailed => "Motor driver initialization failed",
            Self::WatchdogInitFailed => "Watchdog initialization failed",
            Self::SensorReadError => "Sensor read error",
            Self::MotorDriverFault => "Motor driver fault",
            Self::CalibrationFailed => "Calibration failed",
            Self::CommunicationTimeout => "Communication timeout",
            Self::StorageError => "Storage operation failed",
            Self::Overcurrent => "Overcurrent condition",
            Self::Overvoltage => "Overvoltage condition",
            Self::Undervoltage => "Undervoltage condition",
            Self::Overtemperature => "Overtemperature condition",
            Self::InvalidParameter => "Invalid parameter",
            Self::OutOfRange => "Value out of range",
            Self::InvalidConfiguration => "Invalid configuration",
            Self::ControlError => "Control loop error",
            Self::LimitExceeded => "Limit exceeded",
            Self::InvalidState => "Invalid state",
        }
    }
}

/// Firmware result type (alias for convenience).
pub type Result<T> = core::result::Result<T, FirmwareError>;

/// Error collection for tracking multiple initialization errors.
pub struct ErrorCollection {
    errors: heapless::Vec<FirmwareError, 16>,
}

impl ErrorCollection {
    pub const fn new() -> Self {
        Self {
            errors: heapless::Vec::new(),
        }
    }

    pub fn add(&mut self, error: FirmwareError) {
        let _ = self.errors.push(error);
    }

    pub fn is_empty(&self) -> bool {
        self.errors.is_empty()
    }

    pub fn len(&self) -> usize {
        self.errors.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &FirmwareError> {
        self.errors.iter()
    }

    pub fn has_critical_error(&self) -> bool {
        self.errors.iter().any(|e| e.severity() == ErrorSeverity::Critical)
    }

    pub fn max_severity(&self) -> ErrorSeverity {
        self.errors
            .iter()
            .map(|e| e.severity())
            .max()
            .unwrap_or(ErrorSeverity::Info)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_severity_ordering() {
        assert!(ErrorSeverity::Info < ErrorSeverity::Warning);
        assert!(ErrorSeverity::Warning < ErrorSeverity::Error);
        assert!(ErrorSeverity::Error < ErrorSeverity::Critical);
    }

    #[test]
    fn critical_errors_not_recoverable() {
        assert!(!FirmwareError::AdcInitFailed.is_recoverable());
        assert!(!FirmwareError::PwmInitFailed.is_recoverable());
        assert!(!FirmwareError::Overvoltage.is_recoverable());
    }

    #[test]
    fn warning_errors_recoverable() {
        assert!(FirmwareError::UartInitFailed.is_recoverable());
        assert!(FirmwareError::CanInitFailed.is_recoverable());
    }

    #[test]
    fn motor_stop_conditions() {
        assert!(FirmwareError::Overcurrent.requires_motor_stop());
        assert!(FirmwareError::Overvoltage.requires_motor_stop());
        assert!(!FirmwareError::UartInitFailed.requires_motor_stop());
    }

    #[test]
    fn error_collection() {
        let mut errors = ErrorCollection::new();
        assert!(errors.is_empty());

        errors.add(FirmwareError::UartInitFailed);
        errors.add(FirmwareError::AdcInitFailed);

        assert_eq!(errors.len(), 2);
        assert!(errors.has_critical_error());
        assert_eq!(errors.max_severity(), ErrorSeverity::Critical);
    }
}
