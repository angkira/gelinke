"""
AS5047P Magnetic Encoder Mock for Renode
Simulates 14-bit absolute position encoder over SPI
"""

from Antmicro.Renode.Core import *
from Antmicro.Renode.Peripherals.SPI import *
from Antmicro.Renode.Peripherals import *
from Antmicro.Renode.Logging import *


class AS5047PEncoder(ISPIPeripheral):
    """
    Mock AS5047P magnetic position encoder

    SPI Protocol:
    - 16-bit frames
    - Command in upper bits, data in lower 14 bits
    - Provides position, velocity, error status
    """

    def __init__(self):
        self.position = 0  # 14-bit position (0-16383)
        self.velocity = 0  # Simulated velocity
        self.error_flags = 0

        self.last_command = 0

        # Configuration
        self.zero_position = 0
        self.direction = 1  # 1 = forward, -1 = reverse

        self.logger = Logger.GetLogger("AS5047P")
        self.logger.Log(LogLevel.Info, "AS5047P encoder initialized")

    def Reset(self):
        """Reset encoder state"""
        self.position = 0
        self.velocity = 0
        self.error_flags = 0
        self.logger.Log(LogLevel.Info, "Encoder reset")

    def Transmit(self, data):
        """
        Handle SPI transmission

        Commands:
        0x3FFF - Read angle
        0x4001 - Read magnitude
        0x4001 - Read errors
        """
        command = (data >> 14) & 0x3

        if command == 0:  # Read angle
            # Return 14-bit position
            response = self.position & 0x3FFF
            self.logger.Log(LogLevel.Debug, f"Read angle: {self.position}")
            return response

        elif command == 1:  # Read magnitude (for diagnostics)
            # Return constant magnitude
            magnitude = 8000  # Good signal strength
            return magnitude & 0x3FFF

        elif command == 2:  # Read errors
            return self.error_flags & 0x3FFF

        else:
            return 0

    def FinishTransmission(self):
        """Called at end of SPI transaction"""
        pass

    def SetPosition(self, pos):
        """Set encoder position (for testing)"""
        self.position = int(pos) & 0x3FFF
        self.logger.Log(LogLevel.Info, f"Position set to: {self.position}")

    def SetVelocity(self, vel):
        """Set encoder velocity (for simulation)"""
        self.velocity = vel

    def Update(self, dt):
        """Update encoder position based on velocity"""
        # Integrate velocity
        delta = int(
            self.velocity * dt * 16384 / (2 * 3.14159)
        )  # Convert to encoder ticks
        self.position = (self.position + delta) & 0x3FFF
