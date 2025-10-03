/// Transport abstraction layer for iRPC protocol
///
/// This module provides a transport-agnostic interface for iRPC messages,
/// hiding the underlying CAN-FD/USB/SPI implementation details.

use irpc::protocol::{Message, ProtocolError};
use crate::firmware::drivers::can::{CanDriver, CanFrame};

/// Transport layer for iRPC messages over CAN-FD.
///
/// This wraps the low-level CAN driver and handles automatic
/// serialization/deserialization of iRPC messages.
pub struct IrpcTransport<'a> {
    can: &'a mut CanDriver,
}

impl<'a> IrpcTransport<'a> {
    /// Create a new iRPC transport over CAN-FD.
    pub fn new(can: &'a mut CanDriver) -> Self {
        Self { can }
    }

    /// Send an iRPC message over CAN-FD.
    ///
    /// This automatically serializes the message and sends it as CAN frame(s).
    pub async fn send_message(&mut self, msg: &Message) -> Result<(), TransportError> {
        // Serialize message
        let data = msg.serialize()
            .map_err(|e| TransportError::Serialization(e))?;

        // Check if fits in single CAN-FD frame (64 bytes)
        if data.len() <= 64 {
            // Single frame transmission
            let frame = CanFrame::new(self.can.node_id())
                .with_data(&data);
            
            self.can.send(frame).await
                .map_err(|_| TransportError::CanBusError)?;
        } else {
            // TODO: Multi-frame transmission for large messages
            return Err(TransportError::MessageTooLarge(data.len()));
        }

        defmt::trace!("iRPC TX: {} bytes", data.len());
        Ok(())
    }

    /// Receive an iRPC message from CAN-FD.
    ///
    /// This automatically deserializes received CAN frame(s) into iRPC messages.
    /// Returns None if no message available.
    pub async fn receive_message(&mut self) -> Result<Option<Message>, TransportError> {
        // Receive CAN frame
        let frame = match self.can.receive().await {
            Ok(frame) => frame,
            Err(_) => return Ok(None), // No message available
        };

        // Deserialize message
        let msg = Message::deserialize(&frame.data)
            .map_err(|e| TransportError::Deserialization(e))?;

        defmt::trace!("iRPC RX: msg_id={}", msg.header.msg_id);
        Ok(Some(msg))
    }

    /// Get the node ID of this transport.
    pub fn node_id(&self) -> u16 {
        self.can.node_id()
    }
}

/// Transport layer errors.
#[derive(Debug)]
pub enum TransportError {
    /// Message serialization failed
    Serialization(ProtocolError),
    /// Message deserialization failed
    Deserialization(ProtocolError),
    /// CAN bus communication error
    CanBusError,
    /// Message exceeds transport MTU
    MessageTooLarge(usize),
    /// Invalid frame format
    InvalidFrame,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transport_error_debug() {
        let err = TransportError::MessageTooLarge(256);
        let debug_str = format!("{:?}", err);
        assert!(debug_str.contains("MessageTooLarge"));
    }
}

