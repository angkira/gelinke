"""
Current Sense ADC Mock for Renode
Simulates ADC readings for 3-phase current sensing
"""

from Antmicro.Renode.Core import *
from Antmicro.Renode.Peripherals.Analog import *
from Antmicro.Renode.Logging import *


class CurrentSenseADC:
    """
    Mock ADC for current sensing

    Provides:
    - Phase A current
    - Phase B current
    - Phase C current (calculated from A+B+C=0)
    - DC bus voltage
    """

    def __init__(self):
        self.phase_a_current = 0.0  # Amperes
        self.phase_b_current = 0.0
        self.vbus = 24.0  # DC bus voltage

        self.adc_resolution = 12  # 12-bit ADC
        self.adc_vref = 3.3  # ADC reference voltage
        self.current_gain = 50  # mV/A (current sense amplifier)
        self.current_offset = 1.65  # V (half of Vref)

        self.logger = Logger.GetLogger("CurrentADC")
        self.logger.Log(LogLevel.Info, "Current sense ADC initialized")

    def Reset(self):
        """Reset ADC state"""
        self.phase_a_current = 0.0
        self.phase_b_current = 0.0
        self.vbus = 24.0

    def CurrentToADC(self, current_amps):
        """Convert current in Amperes to ADC counts"""
        # Voltage = offset + (current * gain)
        voltage = self.current_offset + (current_amps * self.current_gain / 1000.0)

        # Clamp to ADC range
        voltage = max(0.0, min(self.adc_vref, voltage))

        # Convert to ADC counts
        adc_value = int((voltage / self.adc_vref) * (2**self.adc_resolution - 1))
        return adc_value

    def VoltageToADC(self, voltage):
        """Convert voltage to ADC counts"""
        # Voltage divider for Vbus measurement (e.g., 10:1)
        measured_voltage = voltage / 10.0
        measured_voltage = max(0.0, min(self.adc_vref, measured_voltage))

        adc_value = int(
            (measured_voltage / self.adc_vref) * (2**self.adc_resolution - 1)
        )
        return adc_value

    def ReadChannel(self, channel):
        """Read ADC channel

        Channels:
        0 - Phase A current
        1 - Phase B current
        2 - Vbus voltage
        """
        if channel == 0:
            return self.CurrentToADC(self.phase_a_current)
        elif channel == 1:
            return self.CurrentToADC(self.phase_b_current)
        elif channel == 2:
            return self.VoltageToADC(self.vbus)
        else:
            return 0

    def SetPhaseCurrent(self, phase, current):
        """Set phase current for simulation"""
        if phase == "A":
            self.phase_a_current = current
        elif phase == "B":
            self.phase_b_current = current

        self.logger.Log(LogLevel.Debug, f"Phase {phase} current: {current:.2f}A")

    def SetLoad(self, torque_nm):
        """Simulate motor load (for testing)"""
        # Estimate current from torque
        # Assuming Kt = 0.1 Nm/A
        kt = 0.1
        total_current = torque_nm / kt

        # Distribute across phases (simplified)
        self.phase_a_current = total_current * 0.6
        self.phase_b_current = total_current * 0.4

        self.logger.Log(
            LogLevel.Info, f"Load set: {torque_nm:.2f}Nm -> {total_current:.2f}A"
        )
