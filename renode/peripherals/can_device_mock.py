"""
CAN Device Mock - External CAN device simulator for testing

This peripheral acts as an external CAN device (e.g., arm controller)
that sends iRPC commands to the firmware and receives responses.

Usage in .repl:
    canDevice: Python.PythonPeripheral @ sysbus 0x50000000
        size: 0x1000
        initable: true
        filename: "renode/peripherals/can_device_mock.py"
"""


class CanDeviceMock:
    """
    External CAN device simulator.

    Provides methods to send iRPC commands and receive responses via CAN hub.
    """

    def __init__(self):
        self.can_hub = None
        self.node_id = 0x0000  # Arm/master node ID
        self.target_id = 0x0010  # Joint node ID
        self.msg_id = 0
        self.rx_queue = []
        self.tx_queue = []

    def setup(self, machine):
        """Called by Renode during initialization"""
        self.machine = machine
        self.log.Info("CAN Device Mock initialized")

    def reset(self):
        """Reset device state"""
        self.msg_id = 0
        self.rx_queue.clear()
        self.tx_queue.clear()

    def send_configure(self):
        """Send iRPC Configure command"""
        self.msg_id += 1
        data = [
            0x00,
            0x00,  # source_id (u16 LE)
            0x10,
            0x00,  # target_id (u16 LE)
            self.msg_id & 0xFF,
            0x00,
            0x00,
            0x00,  # msg_id (u32 LE)
            0x01,  # payload variant: Configure
        ]
        self.tx_queue.append(data)
        self.log.Info(f"Queued Configure command (msg_id={self.msg_id})")
        return self.msg_id

    def send_activate(self):
        """Send iRPC Activate command"""
        self.msg_id += 1
        data = [
            0x00,
            0x00,  # source_id
            0x10,
            0x00,  # target_id
            self.msg_id & 0xFF,
            0x00,
            0x00,
            0x00,  # msg_id
            0x02,  # payload variant: Activate
        ]
        self.tx_queue.append(data)
        self.log.Info(f"Queued Activate command (msg_id={self.msg_id})")
        return self.msg_id

    def send_set_target(self, angle_deg, velocity_deg_s):
        """Send iRPC SetTarget command"""
        import struct

        self.msg_id += 1

        # Encode floats as little-endian
        angle_bytes = list(struct.pack("<f", angle_deg))
        velocity_bytes = list(struct.pack("<f", velocity_deg_s))

        data = (
            [
                0x00,
                0x00,  # source_id
                0x10,
                0x00,  # target_id
                self.msg_id & 0xFF,
                0x00,
                0x00,
                0x00,  # msg_id
                0x00,  # payload variant: SetTarget
            ]
            + angle_bytes
            + velocity_bytes
        )

        self.tx_queue.append(data)
        self.log.Info(f"Queued SetTarget({angle_deg}°, {velocity_deg_s}°/s)")
        return self.msg_id

    def get_tx_queue(self):
        """Get pending TX messages"""
        return list(self.tx_queue)

    def clear_tx_queue(self):
        """Clear TX queue"""
        count = len(self.tx_queue)
        self.tx_queue.clear()
        return count

    def receive_message(self, data):
        """Called when CAN message arrives"""
        self.rx_queue.append(data)
        self.log.Debug(f"Received CAN message: {len(data)} bytes")

    def get_last_response(self):
        """Get last received response"""
        if self.rx_queue:
            return self.rx_queue[-1]
        return None

    def has_response(self):
        """Check if response received"""
        return len(self.rx_queue) > 0


# Renode interface
if "request" in dir():
    # This code runs in Renode Python context

    if request.isInit:
        # Initialize mock device
        device = CanDeviceMock()
        device.log = self.Log
        self.device = device
        self.NoisyLog("CAN Device Mock initialized")

    # Handle register access (if needed)
    if request.isRead:
        # Register 0x00: Has response flag
        if request.offset == 0x00:
            request.value = 1 if self.device.has_response() else 0
        # Register 0x04: Message count
        elif request.offset == 0x04:
            request.value = len(self.device.rx_queue)
        else:
            request.value = 0

    if request.isWrite:
        # Register 0x00: Clear RX queue
        if request.offset == 0x00:
            self.device.rx_queue.clear()
            self.NoisyLog("RX queue cleared")
