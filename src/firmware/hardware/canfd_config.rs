//! CAN-FD configuration for iRPC transport integration.
//!
//! This module provides hardware-specific configuration that will be passed
//! to `irpc::transport::CanFdTransport` when it's available.

/// CAN-FD pin numbers for STM32G431CB (CLN17 v2.0).
///
/// Hardware connections:
/// - PA12: FDCAN1_TX (to DRV8844 via CAN transceiver)
/// - PA11: FDCAN1_RX (from DRV8844 via CAN transceiver)
///
/// Note: These are pin identifiers, not owned peripherals.
/// The actual peripheral ownership will be handled by iRPC transport.
#[derive(Clone, Copy, Debug)]
pub struct CanFdPinConfig {
    /// TX pin identifier
    pub tx: PinId,
    /// RX pin identifier
    pub rx: PinId,
}

/// Pin identifier for configuration.
#[derive(Clone, Copy, Debug)]
pub enum PinId {
    PA11,
    PA12,
}

impl Default for CanFdPinConfig {
    fn default() -> Self {
        Self {
            tx: PinId::PA12,
            rx: PinId::PA11,
        }
    }
}

/// CAN-FD bitrate configuration.
///
/// CLN17 v2.0 uses:
/// - 1 Mbps nominal bitrate (for arbitration phase)
/// - 5 Mbps data bitrate (for FD data phase)
#[derive(Clone, Copy, Debug)]
pub struct CanFdBitrates {
    /// Nominal bitrate in Hz (arbitration phase)
    pub nominal: u32,
    /// Data bitrate in Hz (FD data phase)
    pub data: u32,
}

impl Default for CanFdBitrates {
    fn default() -> Self {
        Self {
            nominal: 1_000_000,  // 1 Mbps
            data: 5_000_000,     // 5 Mbps
        }
    }
}

/// Complete CAN-FD configuration for iRPC transport.
///
/// This structure will be passed to `irpc::transport::CanFdTransport::new()`
/// when the new iRPC API is available.
#[derive(Clone, Copy, Debug)]
pub struct CanFdConfig {
    /// CAN node ID (device address on the bus)
    pub node_id: u16,
    
    /// Bitrate configuration
    pub bitrates: CanFdBitrates,
    
    /// Enable loopback mode (for testing without physical bus)
    pub loopback: bool,
    
    /// Enable silent mode (listen-only, no ACK)
    pub silent: bool,
}

impl CanFdConfig {
    /// Create configuration for CLN17 v2.0 joint.
    ///
    /// # Arguments
    /// * `node_id` - Unique joint ID on the CAN bus (0x0001..0xFFFF)
    pub const fn for_joint(node_id: u16) -> Self {
        Self {
            node_id,
            bitrates: CanFdBitrates {
                nominal: 1_000_000,
                data: 5_000_000,
            },
            loopback: false,
            silent: false,
        }
    }
    
    /// Create test configuration with loopback enabled.
    pub const fn for_testing(node_id: u16) -> Self {
        Self {
            node_id,
            bitrates: CanFdBitrates {
                nominal: 1_000_000,
                data: 5_000_000,
            },
            loopback: true,
            silent: false,
        }
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_bitrates() {
        let bitrates = CanFdBitrates::default();
        assert_eq!(bitrates.nominal, 1_000_000);
        assert_eq!(bitrates.data, 5_000_000);
    }

    #[test]
    fn joint_config() {
        let config = CanFdConfig::for_joint(0x0010);
        assert_eq!(config.node_id, 0x0010);
        assert_eq!(config.bitrates.nominal, 1_000_000);
        assert!(!config.loopback);
        assert!(!config.silent);
    }

    #[test]
    fn test_config() {
        let config = CanFdConfig::for_testing(0x0020);
        assert_eq!(config.node_id, 0x0020);
        assert!(config.loopback);
    }
}

