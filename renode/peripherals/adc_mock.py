"""
ADC Mock - Current sensor simulator for testing

Simulates phase current readings for FOC testing.

Usage in .repl:
    adcMock: Python.PythonPeripheral @ sysbus 0x50001000
        size: 0x100
        initable: true
        filename: "renode/peripherals/adc_mock.py"

Register Map:
    0x00: Phase A current (raw ADC value, 0-4095, offset 2048)
    0x04: Phase B current (raw ADC value, 0-4095, offset 2048)
    0x08: Phase C current (raw ADC value, 0-4095, offset 2048)
    0x0C: DC bus voltage (raw ADC value, 0-4095)
    0x10: Control register (bit 0: enable synthetic motion)
"""

import math


class AdcMock:
    """ADC peripheral mock for current sensing"""

    def __init__(self):
        # ADC zero-current offset (mid-scale 12-bit)
        self.offset = 2048

        # Current synthetic values (raw ADC)
        self.phase_a = self.offset
        self.phase_b = self.offset
        self.phase_c = self.offset
        self.dc_voltage = 2731  # ~40V (40/60 * 4095)

        # Synthetic motion parameters
        self.enable_motion = False
        self.angle_rad = 0.0
        self.velocity_rad_s = 0.0
        self.current_amplitude = 100  # ADC counts above offset
        self.time_counter = 0

    def set_phase_current(self, phase, current_amps):
        """
        Set phase current in Amperes.
        Converts to raw ADC value.

        Assuming:
        - Shunt: 10 mOhm
        - Gain: 20 V/V
        - Vref: 3.3V
        - 12-bit ADC (0-4095)

        Formula: ADC = offset + (current_A * shunt_R * gain) / Vref * 4095
        """
        # 10 mOhm shunt, 20x gain → 0.2 V/A
        voltage = current_amps * 0.2
        adc_counts = int((voltage / 3.3) * 4095)
        raw_value = self.offset + adc_counts

        # Clamp to 12-bit range
        raw_value = max(0, min(4095, raw_value))

        if phase == "A":
            self.phase_a = raw_value
        elif phase == "B":
            self.phase_b = raw_value
        elif phase == "C":
            self.phase_c = raw_value

    def set_dc_voltage(self, voltage_v):
        """Set DC bus voltage in Volts (0-60V range)"""
        # Assuming 60V max, 12-bit ADC
        adc_value = int((voltage_v / 60.0) * 4095)
        self.dc_voltage = max(0, min(4095, adc_value))

    def enable_synthetic_motion(self, enable, velocity_rad_s=1.0, amplitude_amps=1.0):
        """
        Enable synthetic 3-phase sinusoidal currents.

        Simulates motor running with given velocity and current amplitude.
        """
        self.enable_motion = enable
        self.velocity_rad_s = velocity_rad_s
        # Convert amplitude to ADC counts
        self.current_amplitude = int(amplitude_amps * 0.2 / 3.3 * 4095)

    def update(self, dt_sec=0.001):
        """Update synthetic motion (called periodically)"""
        if not self.enable_motion:
            return

        # Increment angle
        self.angle_rad += self.velocity_rad_s * dt_sec

        # Generate 3-phase sinusoidal currents
        # Phase A: sin(theta)
        self.phase_a = self.offset + int(
            self.current_amplitude * math.sin(self.angle_rad)
        )

        # Phase B: sin(theta - 120°)
        self.phase_b = self.offset + int(
            self.current_amplitude * math.sin(self.angle_rad - 2.094)
        )

        # Phase C: sin(theta + 120°)
        self.phase_c = self.offset + int(
            self.current_amplitude * math.sin(self.angle_rad + 2.094)
        )

        # Clamp values
        self.phase_a = max(0, min(4095, self.phase_a))
        self.phase_b = max(0, min(4095, self.phase_b))
        self.phase_c = max(0, min(4095, self.phase_c))


# Renode interface
if "request" in dir():
    if request.isInit:
        # Initialize ADC mock
        adc = AdcMock()
        self.adc = adc
        self.NoisyLog("ADC Mock initialized")

    if request.isRead:
        offset = request.offset

        if offset == 0x00:  # Phase A
            request.value = self.adc.phase_a
        elif offset == 0x04:  # Phase B
            request.value = self.adc.phase_b
        elif offset == 0x08:  # Phase C
            request.value = self.adc.phase_c
        elif offset == 0x0C:  # DC voltage
            request.value = self.adc.dc_voltage
        elif offset == 0x10:  # Control register
            request.value = 1 if self.adc.enable_motion else 0
        else:
            request.value = 0

        # Update motion on each read (simulates continuous conversion)
        if self.adc.enable_motion:
            self.adc.update(0.0001)  # 100 µs per read

    if request.isWrite:
        offset = request.offset
        value = request.value

        if offset == 0x00:  # Phase A raw set
            self.adc.phase_a = value & 0xFFF
        elif offset == 0x04:  # Phase B raw set
            self.adc.phase_b = value & 0xFFF
        elif offset == 0x08:  # Phase C raw set
            self.adc.phase_c = value & 0xFFF
        elif offset == 0x0C:  # DC voltage
            self.adc.dc_voltage = value & 0xFFF
        elif offset == 0x10:  # Control: enable synthetic motion
            self.adc.enable_motion = (value & 1) != 0
            if self.adc.enable_motion:
                self.NoisyLog("ADC Mock: Synthetic motion enabled")


