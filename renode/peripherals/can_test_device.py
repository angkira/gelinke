"""
CAN Test Device for Renode
Simulates external CAN device sending iRPC commands and receiving responses
"""

from Antmicro.Renode.Core import *
from Antmicro.Renode.Peripherals.CAN import *
from Antmicro.Renode.Logging import *
import struct


class CANTestDevice(ICAN):
    """
    Mock CAN device for testing iRPC protocol

    Can send:
    - iRPC commands (Configure, Activate, SetTarget, etc.)
    - Receive responses and telemetry
    """

    def __init__(self):
        self.received_frames = []
        self.response_queue = []

        self.device_id = 0x01  # Joint ID
        self.host_id = 0x00  # Host/master ID

        self.logger = Logger.GetLogger("CANTest")
        self.logger.Log(LogLevel.Info, "CAN test device initialized")

    def Reset(self):
        """Reset device state"""
        self.received_frames.clear()
        self.response_queue.clear()

    def OnFrameReceived(self, id, data, dataLength):
        """Handle received CAN frame"""
        frame = {"id": id, "data": list(data[:dataLength]), "length": dataLength}
        self.received_frames.append(frame)

        self.logger.Log(
            LogLevel.Info,
            f"Received frame: ID=0x{id:03X}, Len={dataLength}, "
            + f"Data=[{', '.join(f'0x{b:02X}' for b in frame['data'])}]",
        )

    def SendCommand(self, command_type, payload_data):
        """
        Send iRPC command

        iRPC Frame format:
        Byte 0: Source ID
        Byte 1: Destination ID
        Byte 2-3: Sequence number
        Byte 4: Payload type
        Byte 5+: Payload data
        """
        frame_data = bytearray(64)  # CAN-FD max

        # Header
        frame_data[0] = self.host_id  # Source
        frame_data[1] = self.device_id  # Destination
        frame_data[2] = 0  # Sequence low
        frame_data[3] = 0  # Sequence high
        frame_data[4] = command_type  # Payload type

        # Payload
        if payload_data:
            frame_data[5 : 5 + len(payload_data)] = payload_data

        length = 5 + len(payload_data) if payload_data else 5

        # Send via CAN
        can_id = (self.host_id << 4) | self.device_id
        self.SendFrame(can_id, bytes(frame_data[:length]))

        self.logger.Log(LogLevel.Info, f"Sent command type 0x{command_type:02X}")

    def SendConfigure(self):
        """Send Configure command (payload type 0x01)"""
        self.SendCommand(0x01, None)

    def SendActivate(self):
        """Send Activate command (payload type 0x02)"""
        self.SendCommand(0x02, None)

    def SendReset(self):
        """Send Reset command (payload type 0x03)"""
        self.SendCommand(0x03, None)

    def SendSetTarget(self, angle_deg, velocity_deg_s):
        """Send SetTarget command (payload type 0x05)"""
        # Convert to radians
        angle_rad = angle_deg * 3.14159 / 180.0
        velocity_rad_s = velocity_deg_s * 3.14159 / 180.0

        # Pack as f32 (little-endian)
        payload = struct.pack("<ff", angle_rad, velocity_rad_s)
        self.SendCommand(0x05, payload)

    def SendSetTargetV2(self, angle_deg, max_vel, max_accel, profile_type):
        """Send enhanced SetTargetV2 command (payload type 0x10)"""
        angle_rad = angle_deg * 3.14159 / 180.0
        max_vel_rad = max_vel * 3.14159 / 180.0
        max_accel_rad = max_accel * 3.14159 / 180.0

        # SetTargetV2 payload
        payload = struct.pack(
            "<ffffffff B",
            angle_rad,  # target_angle
            max_vel_rad,  # max_velocity
            0.0,  # target_velocity
            max_accel_rad,  # max_acceleration
            max_accel_rad,  # max_deceleration
            2000.0,  # max_jerk (default)
            0.0,  # max_current
            0.0,  # max_temperature
            profile_type,  # profile (0=Trap, 1=SCurve, 2=Adaptive)
        )

        self.SendCommand(0x10, payload)

    def GetLastResponse(self):
        """Get most recent response frame"""
        if self.received_frames:
            return self.received_frames[-1]
        return None

    def GetResponsePayloadType(self):
        """Get payload type of last response"""
        frame = self.GetLastResponse()
        if frame and len(frame["data"]) > 4:
            return frame["data"][4]
        return None

    def ClearReceived(self):
        """Clear received frame buffer"""
        self.received_frames.clear()
