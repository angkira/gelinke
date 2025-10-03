use embassy_stm32::Peripherals;
use heapless::Vec;

/// CAN-FD bitrate configuration for 1 Mbps nominal / 5 Mbps data.
/// 
/// For 170 MHz SYSCLK and FDCAN kernel clock:
/// - Nominal: 1 Mbps (for arbitration phase)
/// - Data: 5 Mbps (for data phase with FD frames)
const NOMINAL_BITRATE: u32 = 1_000_000;
const DATA_BITRATE: u32 = 5_000_000;

/// Maximum CAN-FD frame data length.
pub const MAX_DATA_LEN: usize = 64;

/// CAN message ID for this joint (configurable).
pub const DEFAULT_NODE_ID: u16 = 0x01;

/// CAN command IDs.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CanCommand {
    /// Set target position (rad * 1000)
    SetPosition = 0x01,
    /// Set target velocity (rad/s * 1000)
    SetVelocity = 0x02,
    /// Set target torque (Nm * 1000)
    SetTorque = 0x03,
    /// Get status
    GetStatus = 0x10,
    /// Get telemetry
    GetTelemetry = 0x11,
    /// Calibrate encoder
    Calibrate = 0x20,
    /// Emergency stop
    EmergencyStop = 0xFF,
}

impl TryFrom<u8> for CanCommand {
    type Error = ();

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(CanCommand::SetPosition),
            0x02 => Ok(CanCommand::SetVelocity),
            0x03 => Ok(CanCommand::SetTorque),
            0x10 => Ok(CanCommand::GetStatus),
            0x11 => Ok(CanCommand::GetTelemetry),
            0x20 => Ok(CanCommand::Calibrate),
            0xFF => Ok(CanCommand::EmergencyStop),
            _ => Err(()),
        }
    }
}

/// CAN message frame.
#[derive(Clone, Debug)]
pub struct CanFrame {
    pub id: u16,
    pub data: Vec<u8, MAX_DATA_LEN>,
}

impl CanFrame {
    /// Create a new CAN frame.
    pub fn new(id: u16) -> Self {
        Self {
            id,
            data: Vec::new(),
        }
    }

    /// Add data to frame.
    pub fn with_data(mut self, data: &[u8]) -> Self {
        self.data.extend_from_slice(data).ok();
        self
    }

    /// Parse command from frame.
    pub fn parse_command(&self) -> Result<CanCommand, ()> {
        if self.data.is_empty() {
            return Err(());
        }
        CanCommand::try_from(self.data[0])
    }

    /// Get i32 parameter from frame at offset.
    pub fn get_i32(&self, offset: usize) -> Option<i32> {
        if self.data.len() < offset + 4 {
            return None;
        }
        let bytes = [
            self.data[offset],
            self.data[offset + 1],
            self.data[offset + 2],
            self.data[offset + 3],
        ];
        Some(i32::from_le_bytes(bytes))
    }
}

/// Legacy CAN-FD driver (deprecated in favor of irpc::transport::CanFdTransport).
///
/// This driver is kept for backward compatibility and low-level CAN operations.
/// For iRPC communication, use `irpc::transport::CanFdTransport` instead,
/// which provides automatic message serialization and hardware configuration.
///
/// **Migration path:**
/// ```no_run
/// // OLD (this module):
/// let can = CanDriver::new(p, node_id);
/// 
/// // NEW (iRPC library):
/// use irpc::transport::CanFdTransport;
/// let config = CanFdConfig::for_joint(node_id);
/// let transport = CanFdTransport::new(p.FDCAN1, p.PA12, p.PA11, config)?;
/// ```
#[deprecated(note = "Use irpc::transport::CanFdTransport instead")]
pub struct CanDriver {
    node_id: u16,
    // Kept for backward compatibility only
}

impl CanDriver {
    /// Create a legacy CAN-FD driver (deprecated).
    ///
    /// **Deprecated:** Use `irpc::transport::CanFdTransport` instead.
    #[deprecated(note = "Use irpc::transport::CanFdTransport::new() instead")]
    pub fn new(_p: Peripherals, node_id: u16) -> Self {
        defmt::warn!("CanDriver is deprecated, use irpc::transport::CanFdTransport");
        
        Self { node_id }
    }

    /// Get node ID.
    pub fn node_id(&self) -> u16 {
        self.node_id
    }

    /// Send a CAN-FD frame.
    pub async fn send(&mut self, frame: CanFrame) -> Result<(), ()> {
        defmt::debug!("CAN TX: id={:04x}, len={}", frame.id, frame.data.len());
        
        // TODO: Actual transmission via FDCAN HAL
        Ok(())
    }

    /// Receive a CAN-FD frame (non-blocking).
    pub async fn receive(&mut self) -> Result<CanFrame, ()> {
        // TODO: Actual reception via FDCAN HAL
        Err(())
    }

    /// Send status response.
    pub async fn send_status(&mut self, state: u8, error_code: u8) -> Result<(), ()> {
        let frame = CanFrame::new(self.node_id)
            .with_data(&[CanCommand::GetStatus as u8, state, error_code]);
        self.send(frame).await
    }

    /// Send telemetry data.
    pub async fn send_telemetry(
        &mut self,
        position: i32,
        velocity: i32,
        current: i16,
        voltage: u16,
    ) -> Result<(), ()> {
        let mut data = Vec::<u8, MAX_DATA_LEN>::new();
        data.push(CanCommand::GetTelemetry as u8).ok();
        data.extend_from_slice(&position.to_le_bytes()).ok();
        data.extend_from_slice(&velocity.to_le_bytes()).ok();
        data.extend_from_slice(&current.to_le_bytes()).ok();
        data.extend_from_slice(&voltage.to_le_bytes()).ok();
        
        let frame = CanFrame { id: self.node_id, data };
        self.send(frame).await
    }
}

// ============================================================================
// NOTE: EmbeddedTransport implementation REMOVED
// ============================================================================
//
// The old `impl EmbeddedTransport for CanDriver` has been removed because
// iRPC library now provides its own `CanFdTransport` that directly manages
// the FDCAN hardware.
//
// Migration guide:
// ```rust
// // OLD (firmware implements transport):
// impl EmbeddedTransport for CanDriver { ... }
// let transport = TransportLayer::new(can_driver);
//
// // NEW (iRPC library provides transport):
// use irpc::transport::CanFdTransport;
// let transport = CanFdTransport::new(p.FDCAN1, p.PA12, p.PA11, config)?;
// ```
//
// This simplifies firmware code and moves hardware complexity into the
// reusable iRPC library where it belongs.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn can_command_conversion() {
        assert_eq!(CanCommand::try_from(0x01), Ok(CanCommand::SetPosition));
        assert_eq!(CanCommand::try_from(0xFF), Ok(CanCommand::EmergencyStop));
        assert_eq!(CanCommand::try_from(0x99), Err(()));
    }

    #[test]
    fn can_frame_parse_command() {
        let frame = CanFrame::new(0x01).with_data(&[0x01, 0x00, 0x00, 0x00]);
        assert_eq!(frame.parse_command(), Ok(CanCommand::SetPosition));
    }

    #[test]
    fn can_frame_get_i32() {
        let mut data = Vec::<u8, MAX_DATA_LEN>::new();
        data.push(0x01).ok(); // Command
        data.extend_from_slice(&12345i32.to_le_bytes()).ok();
        
        let frame = CanFrame { id: 0x01, data };
        assert_eq!(frame.get_i32(1), Some(12345));
    }

    #[test]
    fn node_id_default() {
        assert_eq!(DEFAULT_NODE_ID, 0x01);
    }
}

