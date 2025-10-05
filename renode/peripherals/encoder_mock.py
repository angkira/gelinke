"""
Encoder Mock - TLE5012B magnetic encoder simulator

Simulates TLE5012B angle sensor over SPI.

Usage in .repl:
    encoderMock: Python.PythonPeripheral @ sysbus 0x50002000
        size: 0x100
        initable: true
        filename: "renode/peripherals/encoder_mock.py"

Register Map (for control):
    0x00: Current angle (0-32767, 15-bit)
    0x04: Angular velocity (signed, millidegrees/sec)
    0x08: Control register (bit 0: enable motion, bit 1: inject error)
    0x0C: Error injection type (0=none, 1=CRC, 2=timeout, 3=invalid data)
"""

import math


class EncoderMock:
    """TLE5012B encoder simulator"""

    def __init__(self):
        # 15-bit angle (0-32767)
        self.angle_raw = 0

        # Motion simulation
        self.enable_motion = False
        self.velocity_deg_s = 0.0  # Degrees per second
        self.angle_deg = 0.0

        # Error injection
        self.inject_error = False
        self.error_type = 0  # 0=none, 1=CRC, 2=timeout, 3=invalid

        # TLE5012B SPI protocol
        self.spi_buffer = []
        self.last_command = 0

    def set_angle(self, angle_deg):
        """Set absolute angle in degrees (0-360)"""
        self.angle_deg = angle_deg % 360.0
        # Convert to 15-bit (0-32767)
        self.angle_raw = int((self.angle_deg / 360.0) * 32767)

    def set_velocity(self, velocity_deg_s):
        """Set angular velocity in degrees/second"""
        self.velocity_deg_s = velocity_deg_s

    def enable_motion_simulation(self, enable):
        """Enable continuous rotation"""
        self.enable_motion = enable

    def update(self, dt_sec=0.001):
        """Update angle based on velocity"""
        if self.enable_motion:
            self.angle_deg += self.velocity_deg_s * dt_sec
            self.angle_deg = self.angle_deg % 360.0
            self.angle_raw = int((self.angle_deg / 360.0) * 32767)

    def inject_error_type(self, error_type):
        """
        Inject error into next SPI read.

        error_type:
            0 = No error
            1 = Bad CRC
            2 = Timeout (no response)
            3 = Invalid data
        """
        self.inject_error = True
        self.error_type = error_type

    def read_angle_register(self):
        """
        Simulate TLE5012B ANGLE_VALUE register read.

        Returns 16-bit value with angle in lower 15 bits.
        """
        if self.inject_error:
            if self.error_type == 1:  # Bad CRC
                # Return angle with flipped bits (bad CRC will fail)
                return (self.angle_raw ^ 0x1234) & 0xFFFF
            elif self.error_type == 2:  # Timeout
                return None  # Simulate no response
            elif self.error_type == 3:  # Invalid data
                return 0xFFFF
            self.inject_error = False  # One-shot error

        # Normal operation: return 15-bit angle
        return self.angle_raw & 0x7FFF

    def spi_transfer(self, tx_byte):
        """
        Handle SPI byte transfer.

        TLE5012B protocol:
        - Command: 0x80 0x21 (read ANGLE_VALUE register 0x0020)
        - Response: 2 bytes (16-bit angle)
        """
        self.spi_buffer.append(tx_byte)

        # Detect read command (simplified)
        if len(self.spi_buffer) == 2:
            if self.spi_buffer[0] == 0x80 and self.spi_buffer[1] == 0x21:
                # Read angle command detected
                self.last_command = 0x8021
                self.spi_buffer.clear()
                return 0x00  # Dummy byte

        # Send angle data
        if self.last_command == 0x8021:
            angle = self.read_angle_register()
            if angle is None:
                return 0xFF  # Timeout indicator

            # Send high byte, then low byte
            if len(self.spi_buffer) == 0:
                return (angle >> 8) & 0xFF
            else:
                self.last_command = 0
                self.spi_buffer.clear()
                return angle & 0xFF

        return 0x00  # Default


# Renode interface
if "request" in dir():
    if request.isInit:
        encoder = EncoderMock()
        self.encoder = encoder
        self.NoisyLog("Encoder Mock initialized")

    if request.isRead:
        offset = request.offset

        if offset == 0x00:  # Current angle (15-bit)
            self.encoder.update(0.0001)  # Update on read
            request.value = self.encoder.angle_raw
        elif offset == 0x04:  # Velocity (millidegrees/sec)
            request.value = int(self.encoder.velocity_deg_s * 1000) & 0xFFFFFFFF
        elif offset == 0x08:  # Control register
            value = 0
            if self.encoder.enable_motion:
                value |= 1
            if self.encoder.inject_error:
                value |= 2
            request.value = value
        elif offset == 0x0C:  # Error type
            request.value = self.encoder.error_type
        else:
            request.value = 0

    if request.isWrite:
        offset = request.offset
        value = request.value

        if offset == 0x00:  # Set angle (raw 15-bit)
            self.encoder.angle_raw = value & 0x7FFF
            self.encoder.angle_deg = (value / 32767.0) * 360.0
        elif offset == 0x04:  # Set velocity (millidegrees/sec)
            self.encoder.velocity_deg_s = (value & 0xFFFFFFFF) / 1000.0
            if value & 0x80000000:  # Handle negative
                self.encoder.velocity_deg_s = -((~value + 1) & 0x7FFFFFFF) / 1000.0
        elif offset == 0x08:  # Control register
            self.encoder.enable_motion = (value & 1) != 0
            self.encoder.inject_error = (value & 2) != 0
            if self.encoder.enable_motion:
                self.NoisyLog(
                    f"Encoder Mock: Motion enabled @ {self.encoder.velocity_deg_s}°/s"
                )
        elif offset == 0x0C:  # Error injection type
            self.encoder.error_type = value & 0x3
            if value != 0:
                self.NoisyLog(f"Encoder Mock: Error injection type={value}")
