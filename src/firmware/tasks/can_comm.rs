use embassy_time::{Duration, Timer};

use crate::firmware::drivers::can::{CanCommand, CanFrame};
use crate::firmware::irpc_integration::JointFocBridge;
use irpc::protocol::Message;

/// CAN communication task with iRPC protocol integration.
///
/// Handles incoming iRPC messages over CAN-FD and dispatches to FOC bridge.
#[embassy_executor::task]
pub async fn can_communication(node_id: u16) {
    defmt::info!("iRPC/CAN communication task starting (joint_id=0x{:04x})", node_id);
    
    // Initialize iRPC-FOC bridge
    let mut bridge = JointFocBridge::new(node_id);
    
    // TODO: Initialize CAN driver
    // let mut can = CanDriver::new(p, node_id);
    
    Timer::after(Duration::from_secs(1)).await;
    
    defmt::info!("iRPC joint ready: lifecycle={:?}, max_msg_size={} bytes", 
                 bridge.state(), Message::max_size());
    
    // Main iRPC message processing loop with NEW serialization API
    loop {
        Timer::after(Duration::from_secs(1)).await;
        
        // Production message processing flow:
        // 1. can.receive() -> CanFrame
        // 2. Message::deserialize(&frame.data) -> Result<Message, ProtocolError>
        // 3. bridge.handle_message(&msg) -> Option<Message>
        // 4. response.serialize() -> Result<Vec<u8>, ProtocolError>
        // 5. can.send(CanFrame { data: serialized })
        
        // Example (when CAN is ready):
        /*
        if let Ok(frame) = can.receive().await {
            match Message::deserialize(&frame.data) {
                Ok(msg) => {
                    if let Some(response) = bridge.handle_message(&msg) {
                        match response.serialize() {
                            Ok(data) => {
                                let resp_frame = CanFrame::new(node_id).with_data(&data);
                                can.send(resp_frame).await.ok();
                            }
                            Err(e) => defmt::error!("iRPC serialize error: {:?}", e),
                        }
                    }
                }
                Err(e) => defmt::warn!("iRPC deserialize error: {:?}", e),
            }
        }
        */
    }
}

/// Process incoming CAN command.
///
/// Returns true if command was processed successfully.
pub fn process_command(frame: &CanFrame) -> Result<CommandResponse, ()> {
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

