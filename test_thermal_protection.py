#!/usr/bin/env python3
"""Test thermal protection and current limiting."""

import sys
import numpy as np
from pathlib import Path

# Add demo_visualization module
sys.path.insert(0, str(Path(__file__).parent))

from demo_visualization import HardwareConfig, apply_current_limit, simulate_temperature

def main():
    print('=' * 80)
    print('Hardware Configuration and Thermal Protection Test')
    print('=' * 80)
    print()

    hw_config = HardwareConfig()

    print('📋 Current Limits:')
    print(f'  Continuous: {hw_config.MAX_CONTINUOUS_CURRENT} A')
    print(f'  Peak: {hw_config.MAX_PEAK_CURRENT} A')
    print(f'  Shutdown: {hw_config.THERMAL_SHUTDOWN_CURRENT} A')
    print()

    print('🌡️  Temperature Limits:')
    print(f'  Nominal: {hw_config.TEMP_NOMINAL}°C')
    print(f'  Warning: {hw_config.TEMP_WARNING}°C')
    print(f'  Critical: {hw_config.TEMP_CRITICAL}°C')
    print(f'  Shutdown: {hw_config.TEMP_SHUTDOWN}°C')
    print()

    print('🔥 Derating Configuration:')
    print(f'  Start derating at: {hw_config.DERATING_START_TEMP}°C')
    print(f'  Full derating at: {hw_config.DERATING_FULL_TEMP}°C')
    print(f'  Minimum factor: {hw_config.DERATING_MIN_FACTOR * 100}%')
    print()

    print('⚡ Current Limiting Examples:')
    print('  Requested | Temp (°C) | Limited | Saturated')
    print('  ' + '-' * 50)

    test_cases = [
        (8.0, 25.0),   # Normal temp, moderate current
        (12.0, 25.0),  # Normal temp, high current
        (8.0, 60.0),   # Warning temp, moderate current
        (8.0, 70.0),   # Hot, moderate current
        (8.0, 80.0),   # Critical temp, moderate current
        (8.0, 90.0),   # Shutdown temp
    ]

    for i_req, temp in test_cases:
        i_lim, saturated = apply_current_limit(i_req, temp, hw_config)
        sat_str = 'YES' if saturated else 'NO'
        print(f'  {i_req:6.1f} A  | {temp:6.1f}   | {i_lim:6.2f} A | {sat_str}')

    print()
    print('🔄 Temperature Simulation Example:')
    print('  Testing sustained 5A current over 10 seconds...')

    temp = hw_config.TEMP_NOMINAL
    dt = 0.001  # 1ms steps
    current = 5.0

    print(f'  Time (s) | Current (A) | Temp (°C) | Derating')
    print('  ' + '-' * 50)

    for t in [0, 1, 2, 5, 10]:
        # Simulate temperature rise
        steps = int(t / dt)
        for _ in range(steps):
            temp = simulate_temperature(current, temp, dt, hw_config)

        # Apply current limit
        i_lim, _ = apply_current_limit(current, temp, hw_config)
        derating = (i_lim / current) * 100 if current > 0 else 100

        print(f'  {t:6.1f}   | {current:9.1f}   | {temp:8.1f}  | {derating:6.1f}%')

    print()
    print('=' * 80)
    print('✅ Thermal protection system ready!')
    print('=' * 80)


if __name__ == '__main__':
    main()
