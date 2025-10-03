use embassy_time::{Duration, Timer};

use crate::firmware::irpc_integration::JointFocBridge;
use crate::firmware::transport::IrpcTransport;
use irpc::protocol::Message;

// Legacy imports for backward compatibility
use crate::firmware::drivers::can::CanCommand;

/// CAN communication task with iRPC protocol integration.
///
/// Uses IrpcTransport abstraction to hide all CAN-FD details.
/// The transport layer automatically handles serialization/deserialization.
#[embassy_executor::task]
pub async fn can_communication(node_id: u16) {
    defmt::info!("iRPC/CAN communication task starting (joint_id=0x{:04x})", node_id);
    
    // Initialize iRPC-FOC bridge
    let mut bridge = JointFocBridge::new(node_id);
    
    // TODO: Initialize CAN driver and transport
    // let mut can = CanDriver::new(p, node_id);
    // let mut transport = IrpcTransport::new(&mut can);
    
    Timer::after(Duration::from_secs(1)).await;
    
    defmt::info!("iRPC joint ready: lifecycle={:?}, max_msg_size={} bytes", 
                 bridge.state(), Message::max_size());
    
    // Main iRPC message processing loop using transport abstraction
    // NO manual serialization needed - transport handles it!
    loop {
        Timer::after(Duration::from_secs(1)).await;
        
        // Production code with transport layer (when CAN is ready):
        /*
        // Receive message (transport handles deserialization)
        match transport.receive_message().await {
            Ok(Some(msg)) => {
                // Process message through iRPC bridge
                if let Some(response) = bridge.handle_message(&msg) {
                    // Send response (transport handles serialization)
                    if let Err(e) = transport.send_message(&response).await {
                        defmt::error!("Transport error: {:?}", e);
                    }
                }
            }
            Ok(None) => {
                // No message available, continue
            }
            Err(e) => {
                defmt::warn!("Transport error: {:?}", e);
            }
        }
        */
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

