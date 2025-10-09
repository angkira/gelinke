#!/usr/bin/env python3
"""
Analyze tracking error sources to identify improvement opportunities.
"""

import json
import numpy as np
import matplotlib.pyplot as plt
from pathlib import Path


def analyze_tracking_error(json_file: str):
    """Analyze tracking error profile to identify improvement opportunities."""

    with open(json_file, "r") as f:
        data = json.load(f)

    samples = data["samples"]

    # Extract data
    time = np.array([s["timestamp"] for s in samples])
    position = np.array([s["position"] for s in samples])
    velocity = np.array([s["velocity"] for s in samples])
    target_pos = np.array([s["target_position"] for s in samples])
    target_vel = np.array([s["target_velocity"] for s in samples])

    # Calculate errors
    pos_error = target_pos - position
    vel_error = target_vel - velocity
    pos_error_deg = np.rad2deg(pos_error)

    # Identify phases
    # Phase 1: Acceleration (velocity increasing)
    # Phase 2: Deceleration (velocity decreasing)
    # Phase 3: Settling (near target)

    vel_diff = np.diff(velocity)
    accel_phase = time < 0.4
    decel_phase = (time >= 0.4) & (time < 0.8)
    settling_phase = time >= 0.8

    # Calculate metrics by phase
    phases = [
        ("Acceleration", accel_phase),
        ("Deceleration", decel_phase),
        ("Settling", settling_phase),
    ]

    print("=" * 80)
    print("TRACKING ERROR ANALYSIS")
    print("=" * 80)

    print(f"\nOverall Statistics:")
    print(f"  RMS error: {np.rad2deg(np.sqrt(np.mean(pos_error**2))):.3f}°")
    print(f"  Max error: {np.max(np.abs(pos_error_deg)):.3f}°")
    print(f"  Mean error: {np.mean(np.abs(pos_error_deg)):.3f}°")
    print(f"  Std dev: {np.std(pos_error_deg):.3f}°")

    print(f"\nError by Phase:")
    for phase_name, phase_mask in phases:
        if np.sum(phase_mask) > 0:
            phase_pos_error = pos_error_deg[phase_mask]
            phase_vel_error = vel_error[phase_mask]

            print(f"\n  {phase_name}:")
            print(f"    Position RMS: {np.sqrt(np.mean(phase_pos_error**2)):.3f}°")
            print(f"    Position Max: {np.max(np.abs(phase_pos_error)):.3f}°")
            print(f"    Position Mean: {np.mean(np.abs(phase_pos_error)):.3f}°")
            print(f"    Velocity RMS: {np.sqrt(np.mean(phase_vel_error**2)):.3f} rad/s")
            print(f"    Velocity Max: {np.max(np.abs(phase_vel_error)):.3f} rad/s")

    # Find where error is largest
    max_error_idx = np.argmax(np.abs(pos_error_deg))
    print(f"\nMax Error Location:")
    print(f"  Time: {time[max_error_idx]:.3f} s")
    print(
        f"  Position: {position[max_error_idx]:.3f} rad ({np.rad2deg(position[max_error_idx]):.1f}°)"
    )
    print(
        f"  Target: {target_pos[max_error_idx]:.3f} rad ({np.rad2deg(target_pos[max_error_idx]):.1f}°)"
    )
    print(f"  Error: {pos_error_deg[max_error_idx]:.3f}°")
    print(f"  Velocity: {velocity[max_error_idx]:.3f} rad/s")
    print(f"  Target velocity: {target_vel[max_error_idx]:.3f} rad/s")

    # Analyze lag
    # Find phase lag by cross-correlation
    if len(position) > 100:
        correlation = np.correlate(
            target_pos - np.mean(target_pos), position - np.mean(position), mode="full"
        )
        lag_idx = np.argmax(correlation) - (len(position) - 1)
        lag_time = lag_idx * (time[1] - time[0]) if len(time) > 1 else 0
        print(f"\nPhase Lag:")
        print(f"  Lag: {lag_idx} samples ({lag_time*1000:.1f} ms)")

    # Steady-state error
    final_100 = pos_error_deg[-100:]
    print(f"\nSteady-State (final 100 samples):")
    print(f"  Mean error: {np.mean(final_100):.3f}°")
    print(f"  RMS error: {np.sqrt(np.mean(final_100**2)):.3f}°")
    print(f"  Max error: {np.max(np.abs(final_100)):.3f}°")

    # Identify issues
    print("\n" + "=" * 80)
    print("IDENTIFIED ISSUES")
    print("=" * 80)

    issues = []

    # Check for excessive lag
    if lag_idx > 5:
        issues.append(
            f"⚠️  Excessive phase lag: {lag_idx} samples ({lag_time*1000:.1f} ms)"
        )
        issues.append("   → Solution: Increase feedforward gain or reduce P gain")

    # Check for steady-state error
    if np.abs(np.mean(final_100)) > 0.5:
        issues.append(f"⚠️  Steady-state error present: {np.mean(final_100):.3f}°")
        issues.append("   → Solution: Increase integral gain (ki_vel)")

    # Check for oscillations
    zero_crossings = np.sum(np.diff(np.sign(pos_error_deg[settling_phase])) != 0)
    if zero_crossings > 5:
        issues.append(
            f"⚠️  Oscillations in settling phase: {zero_crossings} zero crossings"
        )
        issues.append("   → Solution: Increase derivative gain or reduce P/I gains")

    # Check for large transient errors
    max_transient = np.max(np.abs(pos_error_deg[accel_phase | decel_phase]))
    if max_transient > 5:
        issues.append(f"⚠️  Large transient error: {max_transient:.3f}°")
        issues.append("   → Solution: Add acceleration feedforward or increase P gain")

    if not issues:
        issues.append("✅ No major issues identified")

    for issue in issues:
        print(issue)

    return {
        "overall_rms": np.rad2deg(np.sqrt(np.mean(pos_error**2))),
        "max_error": np.max(np.abs(pos_error_deg)),
        "lag_samples": lag_idx if len(position) > 100 else 0,
        "steady_state_error": np.mean(final_100),
        "issues": issues,
    }


if __name__ == "__main__":
    json_file = "demo_results/demo_trapezoidal_profile.json"

    if Path(json_file).exists():
        results = analyze_tracking_error(json_file)
    else:
        print(f"Error: {json_file} not found. Run demo_visualization.py first.")
