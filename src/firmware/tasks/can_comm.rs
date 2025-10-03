use embassy_time::{Duration, Timer};

use crate::firmware::hardware::canfd_config::CanFdConfig;
use crate::firmware::irpc_integration::JointFocBridge;
use irpc::Message;

// Legacy imports for backward compatibility
use crate::firmware::drivers::can::CanCommand;

/// CAN communication task with iRPC protocol integration.
///
/// **NEW ARCHITECTURE:**
/// Uses `irpc::transport::CanFdTransport` - iRPC library OWNS the hardware!
/// 
/// The iRPC library now:
/// - Configures FDCAN peripheral directly (via PAC)
/// - Manages message serialization/deserialization
/// - Handles CAN frame TX/RX
/// - Provides simple typed Message API
///
/// Firmware just provides:
/// - Hardware configuration (pins, bitrates)
/// - Business logic (JointFocBridge)
///
/// **This is the CLEANEST possible embedded communication code!** 🎯
#[embassy_executor::task]
pub async fn can_communication(node_id: u16) {
    defmt::info!("iRPC/CAN communication task starting (joint_id=0x{:04x})", node_id);
    
    // Initialize iRPC-FOC bridge (business logic)
    let mut bridge = JointFocBridge::new(node_id);
    
    // TODO: When iRPC CanFdTransport is ready, replace this with:
    /*
    use irpc::transport::CanFdTransport;
    
    // 1. Configuration (declarative, no hardware knowledge needed!)
    let config = CanFdConfig::for_joint(node_id);
    
    // 2. iRPC creates and manages the transport
    //    (takes ownership of peripherals and configures everything)
    let mut transport = CanFdTransport::new(
        p.FDCAN1,  // FDCAN peripheral
        p.PA12,    // TX pin
        p.PA11,    // RX pin
        config,    // Bitrates, node_id, etc
    ).expect("FDCAN init failed");
    
    defmt::info!("iRPC CAN-FD transport ready: node_id=0x{:04x}, {} Mbps/{} Mbps",
                 node_id,
                 config.bitrates.nominal / 1_000_000,
                 config.bitrates.data / 1_000_000);
    
    // 3. Super simple message loop - just 3 lines!
    loop {
        // Receive (iRPC deserializes automatically)
        if let Ok(Some(msg)) = transport.receive_message() {
            // Handle (pure business logic)
            if let Some(response) = bridge.handle_message(&msg) {
                // Send (iRPC serializes automatically)
                transport.send_message(&response).ok();
            }
        }
        
        // Small yield to prevent blocking
        Timer::after_micros(10).await;
    }
    */
    
    // Temporary: heartbeat until iRPC transport is ready
    Timer::after(Duration::from_secs(1)).await;
    defmt::info!("iRPC joint ready: lifecycle={:?}, awaiting CanFdTransport", 
                 bridge.state());
    
    loop {
        Timer::after(Duration::from_secs(5)).await;
        defmt::info!("Waiting for irpc::transport::CanFdTransport implementation...");
    }
}

/// Legacy process_command function (deprecated - use transport abstraction instead).
///
/// This function is kept for backward compatibility.
#[allow(dead_code)]
pub fn process_command(frame: &crate::firmware::drivers::can::CanFrame) -> Result<CommandResponse, ()> {
    let cmd = frame.parse_command()?;
    
    match cmd {
        CanCommand::SetPosition => {
            let position = frame.get_i32(1).ok_or(())?;
            defmt::info!("CAN cmd: SetPosition({})", position);
            Ok(CommandResponse::Ok)
        }
        
        CanCommand::SetVelocity => {
            let velocity = frame.get_i32(1).ok_or(())?;
            defmt::info!("CAN cmd: SetVelocity({})", velocity);
            Ok(CommandResponse::Ok)
        }
        
        CanCommand::SetTorque => {
            let torque = frame.get_i32(1).ok_or(())?;
            defmt::info!("CAN cmd: SetTorque({})", torque);
            Ok(CommandResponse::Ok)
        }
        
        CanCommand::GetStatus => {
            defmt::debug!("CAN cmd: GetStatus");
            Ok(CommandResponse::Status { state: 1, error: 0 })
        }
        
        CanCommand::GetTelemetry => {
            defmt::debug!("CAN cmd: GetTelemetry");
            Ok(CommandResponse::Telemetry {
                position: 0,
                velocity: 0,
                current: 0,
                voltage: 0,
            })
        }
        
        CanCommand::Calibrate => {
            defmt::info!("CAN cmd: Calibrate");
            Ok(CommandResponse::Ok)
        }
        
        CanCommand::EmergencyStop => {
            defmt::warn!("CAN cmd: EmergencyStop");
            Ok(CommandResponse::Ok)
        }
    }
}

/// Command response types.
#[derive(Debug, PartialEq)]
pub enum CommandResponse {
    Ok,
    Status { state: u8, error: u8 },
    Telemetry {
        position: i32,
        velocity: i32,
        current: i16,
        voltage: u16,
    },
    Error,
}

#[cfg(test)]
mod tests {
    use super::*;
    use heapless::Vec;

    #[test]
    fn process_set_position() {
        let mut data = Vec::new();
        data.push(CanCommand::SetPosition as u8).ok();
        data.extend_from_slice(&1000i32.to_le_bytes()).ok();
        
        let frame = CanFrame { id: 0x01, data };
        let response = process_command(&frame);
        
        assert_eq!(response, Ok(CommandResponse::Ok));
    }

    #[test]
    fn process_get_status() {
        let frame = CanFrame::new(0x01)
            .with_data(&[CanCommand::GetStatus as u8]);
        let response = process_command(&frame);
        
        assert!(matches!(response, Ok(CommandResponse::Status { .. })));
    }

    #[test]
    fn process_emergency_stop() {
        let frame = CanFrame::new(0x01)
            .with_data(&[CanCommand::EmergencyStop as u8]);
        let response = process_command(&frame);
        
        assert_eq!(response, Ok(CommandResponse::Ok));
    }

    #[test]
    fn process_invalid_command() {
        let frame = CanFrame::new(0x01).with_data(&[0x99]);
        let response = process_command(&frame);
        
        assert_eq!(response, Err(()));
    }
}

