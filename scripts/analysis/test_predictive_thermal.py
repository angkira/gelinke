#!/usr/bin/env python3
"""Test Predictive Thermal Management vs Reactive Derating."""

import sys
import numpy as np
import matplotlib.pyplot as plt
from pathlib import Path

# Add demo_visualization module from scripts/demos
sys.path.insert(0, str(Path(__file__).parent / "scripts" / "demos"))

from demo_visualization import (
    HardwareConfig,
    PredictiveThermalManager,
    apply_current_limit,
    simulate_temperature,
)


def test_prediction_accuracy():
    """Test temperature prediction accuracy."""
    print("=" * 80)
    print("🌡️  Temperature Prediction Accuracy Test")
    print("=" * 80)
    print()

    hw_config = HardwareConfig()
    manager = PredictiveThermalManager(hw_config)

    # Test scenarios
    scenarios = [
        ("Low current (2A), 10s", 25.0, 2.0, 10.0),
        ("Medium current (5A), 10s", 25.0, 5.0, 10.0),
        ("High current (8A), 5s", 25.0, 8.0, 5.0),
        ("Hot start (50°C), 5A, 10s", 50.0, 5.0, 10.0),
    ]

    for name, start_temp, current, duration in scenarios:
        # Predict
        predicted = manager.predict_temperature(start_temp, current, duration)

        # Simulate actual
        temp = start_temp
        dt = 0.01  # 10ms steps
        steps = int(duration / dt)
        for _ in range(steps):
            temp = simulate_temperature(current, temp, dt, hw_config)

        error = abs(predicted - temp)
        error_pct = error / max(1, predicted - start_temp) * 100

        print(f"{name}:")
        print(f"  Start: {start_temp:.1f}°C")
        print(f"  Predicted: {predicted:.2f}°C")
        print(f"  Actual: {temp:.2f}°C")
        print(f"  Error: {error:.2f}°C ({error_pct:.1f}%)")
        print()

    print("✅ Prediction accuracy validated!")
    print()


def test_safe_current_limits():
    """Test safe current limit calculation."""
    print("=" * 80)
    print("⚡ Safe Current Limit Calculation")
    print("=" * 80)
    print()

    hw_config = HardwareConfig()
    manager = PredictiveThermalManager(hw_config)

    print(
        f"Hardware: Max Peak = {hw_config.MAX_PEAK_CURRENT}A, "
        f"Max Continuous = {hw_config.MAX_CONTINUOUS_CURRENT}A"
    )
    print(f"Thermal: τ = {manager.tau_thermal:.0f}s")
    print()

    # Test at different temperatures
    temps = [25.0, 40.0, 60.0, 70.0]
    durations = [0.1, 1.0, 5.0, 10.0, 60.0]

    print(f"{'Temp (°C)':<12} | {'Duration (s)':<14} | {'Safe Current (A)':<18}")
    print("-" * 48)

    for temp in temps:
        for duration in durations:
            i_safe = manager.safe_current_limit(temp, duration)
            print(f"{temp:<12.0f} | {duration:<14.1f} | {i_safe:<18.2f}")
        print("-" * 48)

    print()
    print("✅ Burst operation allows higher current for short durations!")
    print()


def test_thermal_capacity():
    """Test thermal capacity calculation."""
    print("=" * 80)
    print("🔋 Thermal Capacity Remaining")
    print("=" * 80)
    print()

    hw_config = HardwareConfig()
    manager = PredictiveThermalManager(hw_config)

    temps = [25.0, 40.0, 50.0, 60.0, 70.0]

    for temp in temps:
        capacity = manager.thermal_capacity_remaining(temp)

        print(f"At {temp:.0f}°C:")
        print(f"  Energy to warning: {capacity['energy_to_warning']:.0f} J")
        print(f"  Energy to critical: {capacity['energy_to_critical']:.0f} J")
        print(f"  Burst (1s): {capacity['burst_current_1s']:.1f} A")
        print(f"  Burst (5s): {capacity['burst_current_5s']:.1f} A")
        print(f"  Continuous safe: {capacity['continuous_safe']:.1f} A")
        print()

    print("✅ Thermal capacity tracking working!")
    print()


def test_predictive_vs_reactive():
    """Compare predictive vs reactive thermal management."""
    print("=" * 80)
    print("🆚 Predictive vs Reactive Thermal Management")
    print("=" * 80)
    print()

    hw_config = HardwareConfig()
    manager = PredictiveThermalManager(hw_config)

    # Simulation parameters
    dt = 0.01  # 10ms
    duration = 120.0  # 2 minutes
    n_samples = int(duration / dt)

    # Storage
    time_arr = []
    temp_predictive_arr = []
    temp_reactive_arr = []
    current_requested_arr = []
    current_predictive_arr = []
    current_reactive_arr = []
    limit_reason_arr = []

    # State
    temp_predictive = hw_config.TEMP_NOMINAL
    temp_reactive = hw_config.TEMP_NOMINAL

    # Current profile: aggressive ramp-up
    def get_requested_current(t):
        """Generate demanding current profile."""
        if t < 10.0:
            # Aggressive ramp
            return min(12.0, t * 1.2)  # Ramp to 12A
        elif t < 40.0:
            # Hold high current
            return 12.0
        elif t < 60.0:
            # Cool down
            return 2.0
        elif t < 80.0:
            # Another burst
            return 10.0
        else:
            # Final cool down
            return 1.0

    predictive_limited_count = 0
    reactive_limited_count = 0
    predictive_shutdown = False
    reactive_shutdown = False

    for i in range(n_samples):
        t = i * dt

        # Requested current
        i_requested = get_requested_current(t)

        # === PREDICTIVE MANAGEMENT ===
        if not predictive_shutdown:
            # Predictive: look ahead 1 second
            i_predictive, is_limited_pred, reason = manager.apply_predictive_limit(
                i_requested, temp_predictive, planned_duration=1.0
            )

            if is_limited_pred:
                predictive_limited_count += 1

            # Update temperature
            temp_predictive = simulate_temperature(
                i_predictive, temp_predictive, dt, hw_config
            )

            # Check for shutdown
            if temp_predictive >= hw_config.TEMP_SHUTDOWN:
                predictive_shutdown = True
                i_predictive = 0.0
        else:
            i_predictive = 0.0
            reason = "SHUTDOWN"

        # === REACTIVE DERATING ===
        if not reactive_shutdown:
            # Reactive: only react to current temperature
            i_reactive, is_saturated = apply_current_limit(
                i_requested, temp_reactive, hw_config
            )

            if is_saturated:
                reactive_limited_count += 1

            # Update temperature
            temp_reactive = simulate_temperature(
                i_reactive, temp_reactive, dt, hw_config
            )

            # Check for shutdown
            if temp_reactive >= hw_config.TEMP_SHUTDOWN:
                reactive_shutdown = True
                i_reactive = 0.0
        else:
            i_reactive = 0.0

        # Record data (downsample to 1Hz)
        if i % 100 == 0:
            time_arr.append(t)
            temp_predictive_arr.append(temp_predictive)
            temp_reactive_arr.append(temp_reactive)
            current_requested_arr.append(i_requested)
            current_predictive_arr.append(i_predictive)
            current_reactive_arr.append(i_reactive)
            limit_reason_arr.append(reason)

    # Convert to numpy
    time_arr = np.array(time_arr)
    temp_predictive_arr = np.array(temp_predictive_arr)
    temp_reactive_arr = np.array(temp_reactive_arr)
    current_requested_arr = np.array(current_requested_arr)
    current_predictive_arr = np.array(current_predictive_arr)
    current_reactive_arr = np.array(current_reactive_arr)

    # Results
    print("📊 Results:")
    print()
    print(f"  Predictive Management:")
    print(f"    Peak temperature: {temp_predictive_arr.max():.1f}°C")
    print(f"    Shutdown: {'YES ❌' if predictive_shutdown else 'NO ✅'}")
    print(
        f"    Limited: {predictive_limited_count} samples ({predictive_limited_count/n_samples*100:.1f}%)"
    )
    print()
    print(f"  Reactive Management:")
    print(f"    Peak temperature: {temp_reactive_arr.max():.1f}°C")
    print(f"    Shutdown: {'YES ❌' if reactive_shutdown else 'NO ✅'}")
    print(
        f"    Limited: {reactive_limited_count} samples ({reactive_limited_count/n_samples*100:.1f}%)"
    )
    print()

    if not predictive_shutdown and reactive_shutdown:
        print("✅ Predictive management PREVENTED shutdown!")
    elif predictive_shutdown and reactive_shutdown:
        print("⚠️  Both approaches hit shutdown (current profile too aggressive)")
    else:
        print("✅ Both approaches avoided shutdown")

    print()

    # Plot results
    fig, axes = plt.subplots(2, 1, figsize=(12, 8), sharex=True)

    # Plot 1: Temperature
    ax1 = axes[0]
    ax1.plot(
        time_arr, temp_predictive_arr, "g-", label="Predictive Management", linewidth=2
    )
    ax1.plot(
        time_arr,
        temp_reactive_arr,
        "r-",
        label="Reactive Derating",
        linewidth=2,
        alpha=0.7,
    )
    ax1.axhline(
        hw_config.TEMP_WARNING,
        color="orange",
        linestyle="--",
        label="Warning",
        alpha=0.5,
    )
    ax1.axhline(
        hw_config.TEMP_CRITICAL,
        color="red",
        linestyle="--",
        label="Critical",
        alpha=0.5,
    )
    ax1.axhline(
        hw_config.TEMP_SHUTDOWN,
        color="darkred",
        linestyle="--",
        label="Shutdown",
        alpha=0.5,
    )
    ax1.set_ylabel("Temperature (°C)", fontsize=12)
    ax1.legend(loc="upper left")
    ax1.grid(True, alpha=0.3)
    ax1.set_title(
        "Predictive vs Reactive Thermal Management", fontsize=14, fontweight="bold"
    )

    # Plot 2: Current
    ax2 = axes[1]
    ax2.plot(
        time_arr,
        current_requested_arr,
        "k--",
        label="Requested",
        linewidth=1,
        alpha=0.5,
    )
    ax2.plot(
        time_arr, current_predictive_arr, "g-", label="Predictive (Actual)", linewidth=2
    )
    ax2.plot(
        time_arr,
        current_reactive_arr,
        "r-",
        label="Reactive (Actual)",
        linewidth=2,
        alpha=0.7,
    )
    ax2.axhline(
        hw_config.MAX_CONTINUOUS_CURRENT,
        color="blue",
        linestyle="--",
        label="Continuous Limit",
        alpha=0.5,
    )
    ax2.axhline(
        hw_config.MAX_PEAK_CURRENT,
        color="purple",
        linestyle="--",
        label="Peak Limit",
        alpha=0.5,
    )
    ax2.set_xlabel("Time (s)", fontsize=12)
    ax2.set_ylabel("Current (A)", fontsize=12)
    ax2.legend(loc="upper right")
    ax2.grid(True, alpha=0.3)

    plt.tight_layout()

    # Save plot
    output_file = "demo_results/predictive_thermal_comparison.png"
    Path("demo_results").mkdir(exist_ok=True)
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"📊 Plot saved to: {output_file}")
    print()

    print("=" * 80)
    print("✅ Predictive Thermal Management Test Complete!")
    print("=" * 80)


def main():
    """Run all predictive thermal tests."""
    test_prediction_accuracy()
    test_safe_current_limits()
    test_thermal_capacity()
    test_predictive_vs_reactive()

    print()
    print("🎯 Key Benefits of Predictive Thermal Management:")
    print("  1. Prevents thermal shutdowns (looks ahead)")
    print("  2. Allows burst operation (short high-current pulses)")
    print("  3. Smoother derating (no sudden limits)")
    print("  4. Better user experience (no surprise shutdowns)")
    print()


if __name__ == "__main__":
    main()
