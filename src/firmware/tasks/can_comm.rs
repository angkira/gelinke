use embassy_time::Timer;
use embassy_stm32::{bind_interrupts, can, peripherals};

use crate::firmware::irpc_integration::JointFocBridge;
use irpc::transport::{CanFdTransport, CanFdConfig};

// Legacy imports for backward compatibility
use crate::firmware::drivers::can::CanCommand;

// Bind FDCAN interrupts
bind_interrupts!(struct Irqs {
    FDCAN1_IT0 => can::IT0InterruptHandler<peripherals::FDCAN1>;
    FDCAN1_IT1 => can::IT1InterruptHandler<peripherals::FDCAN1>;
});

/// CAN communication task with iRPC protocol integration.
///
/// **PRODUCTION READY!** ✅
/// Uses `irpc::transport::CanFdTransport` - iRPC library OWNS the hardware!
/// 
/// The iRPC library:
/// - Configures FDCAN peripheral directly (via PAC)
/// - Manages message serialization/deserialization
/// - Handles CAN frame TX/RX
/// - Provides simple typed Message API
///
/// Firmware provides:
/// - Hardware configuration (pins, bitrates)
/// - Business logic (JointFocBridge)
///
/// **This is the CLEANEST possible embedded communication code!** 🎯
#[embassy_executor::task]
pub async fn can_communication(
    node_id: u16,
    fdcan: embassy_stm32::Peri<'static, peripherals::FDCAN1>,
    tx_pin: embassy_stm32::Peri<'static, peripherals::PA12>,
    rx_pin: embassy_stm32::Peri<'static, peripherals::PA11>,
) {
    defmt::info!("iRPC/CAN communication task starting (joint_id=0x{:04x})", node_id);
    
    // Initialize iRPC-FOC bridge (business logic)
    let mut bridge = JointFocBridge::new(node_id);
    
    // 1. Configuration (declarative, no hardware knowledge needed!)
    let config = CanFdConfig {
        node_id,
        nominal_bitrate: 1_000_000,  // 1 Mbps
        data_bitrate: 5_000_000,     // 5 Mbps
    };
    
    // 2. iRPC creates and manages the transport
    //    (takes ownership of peripherals and configures everything)
    let mut transport = match CanFdTransport::new(fdcan, rx_pin, tx_pin, Irqs, config) {
        Ok(t) => {
            defmt::info!("✅ CAN-FD transport ready: node_id=0x{:04x}, 1 Mbps / 5 Mbps", node_id);
            t
        }
        Err(e) => {
            defmt::error!("❌ FDCAN init failed: {:?}", e);
            panic!("Cannot initialize CAN-FD transport");
        }
    };
    
    defmt::info!("iRPC joint ready: lifecycle={:?}", bridge.state());
    
    // 3. Super simple message loop - just 3 lines!
    loop {
        // Receive (iRPC deserializes automatically)
        if let Ok(msg) = transport.receive_message().await {
            defmt::trace!("RX: msg_id={}", msg.header.msg_id);
            
            // Handle (pure business logic)
            if let Some(response) = bridge.handle_message(&msg) {
                defmt::trace!("TX: response msg_id={}", response.header.msg_id);
                
                // Send (iRPC serializes automatically)
                if let Err(e) = transport.send_message(&response).await {
                    defmt::error!("TX failed: {:?}", e);
                }
            }
        }
        
        // Small yield to prevent busy-waiting
        Timer::after_micros(10).await;
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

