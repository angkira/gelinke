#!/usr/bin/env python3
"""
Optimize Controller Gains for S-curve Trajectory

Test different controller gains specifically optimized for S-curve trajectories.
"""

import sys
import numpy as np
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "renode" / "tests"))

from compare_trajectories import simulate_motion_comparison


def optimize_scurve_controller():
    """Test different controller gains for S-curve trajectory."""
    print("=" * 80)
    print("CONTROLLER OPTIMIZATION FOR S-CURVE TRAJECTORY")
    print("=" * 80)
    print("\nOptimal jerk from previous test: 100 rad/s³")
    print("Goal: Reduce RMS error from 2.670° to < 1°\n")

    max_jerk = 100.0  # Optimal from previous test

    # Test configurations with varying integral gains and feedforward
    configs = [
        {
            "name": "Baseline (Option 14)",
            "ki_vel": 1.5,
            "kff_accel": 0.3,
        },
        {
            "name": "Higher Integral (ki=2.0)",
            "ki_vel": 2.0,
            "kff_accel": 0.3,
        },
        {
            "name": "Much Higher Integral (ki=2.5)",
            "ki_vel": 2.5,
            "kff_accel": 0.3,
        },
        {
            "name": "Higher Integral + More FF (ki=2.0, kff=0.4)",
            "ki_vel": 2.0,
            "kff_accel": 0.4,
        },
        {
            "name": "Higher Integral + More FF (ki=2.5, kff=0.4)",
            "ki_vel": 2.5,
            "kff_accel": 0.4,
        },
        {
            "name": "Aggressive (ki=3.0, kff=0.4)",
            "ki_vel": 3.0,
            "kff_accel": 0.4,
        },
        {
            "name": "Very Aggressive (ki=3.5, kff=0.5)",
            "ki_vel": 3.5,
            "kff_accel": 0.5,
        },
    ]

    results = []

    for config in configs:
        print(f"Testing: {config['name']}...")

        metrics = simulate_motion_comparison(
            trajectory_type="scurve",
            kp_pos=6.0,
            kp_vel=3.5,
            ki_vel=config["ki_vel"],
            kd_vel=1.0,
            kff_vel=1.0,
            kff_accel=config["kff_accel"],
            max_jerk=max_jerk,
        )

        config["metrics"] = metrics
        results.append(config)

        print(f"  RMS error: {metrics['rms_error_deg']:.3f}°")
        print(f"  Max error: {metrics['max_error_deg']:.3f}°")
        print(f"  Overshoot: {metrics['overshoot_percent']:.1f}%")
        print(f"  Settling RMS: {metrics['settling_rms_deg']:.3f}°")
        print()

    # Comparison table
    print("=" * 80)
    print("RESULTS")
    print("=" * 80)
    print()
    print(f"{'Configuration':<40} {'RMS°':>8} {'Max°':>8} {'OS%':>6} {'Settle°':>9}")
    print("-" * 80)

    baseline = results[0]["metrics"]

    for r in results:
        m = r["metrics"]
        improvement = (baseline["rms_error_deg"] - m["rms_error_deg"]) / baseline["rms_error_deg"] * 100

        status = ""
        if m["rms_error_deg"] < 1.0:
            status = " ✅"
        elif m["overshoot_percent"] > 10:
            status = " ❌ OS"

        print(
            f"{r['name']:<40} "
            f"{m['rms_error_deg']:>8.3f} "
            f"{m['max_error_deg']:>8.3f} "
            f"{m['overshoot_percent']:>6.1f} "
            f"{m['settling_rms_deg']:>9.3f}{status}"
        )
        if r != results[0]:
            print(f"  → Change: {improvement:+.1f}%")

    # Find best
    print()
    print("=" * 80)
    print("BEST CONFIGURATION")
    print("=" * 80)
    print()

    # Filter out configs with excessive overshoot
    valid = [r for r in results if r["metrics"]["overshoot_percent"] < 10]

    if valid:
        best = min(valid, key=lambda x: x["metrics"]["rms_error_deg"])
        m = best["metrics"]

        print(f"🏆 {best['name']}")
        print(f"  ki_vel = {best['ki_vel']}")
        print(f"  kff_accel = {best['kff_accel']}")
        print()
        print("Performance:")
        print(f"  RMS Error:        {m['rms_error_deg']:.3f}°")
        print(f"  Max Error:        {m['max_error_deg']:.3f}°")
        print(f"  Overshoot:        {m['overshoot_percent']:.1f}%")
        print(f"  Settling RMS:     {m['settling_rms_deg']:.3f}°")
        print()
        print("Improvement from Baseline:")
        print(f"  RMS: {baseline['rms_error_deg']:.3f}° → {m['rms_error_deg']:.3f}° "
              f"({(baseline['rms_error_deg'] - m['rms_error_deg']) / baseline['rms_error_deg'] * 100:.1f}% better)")
        print()

        if m["rms_error_deg"] < 1.0:
            print("🎯 TARGET ACHIEVED: RMS < 1°! ✅")
        else:
            print(f"⚠️  Still {m['rms_error_deg'] - 1.0:.3f}° away from <1° target")

    print()
    print("=" * 80)

    return results


if __name__ == "__main__":
    optimize_scurve_controller()
