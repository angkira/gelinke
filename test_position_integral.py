#!/usr/bin/env python3
"""
Test Position Integral Term to Fix Steady-State Error

Compares S-curve performance with and without position integral gain.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "scripts" / "analysis"))

from compare_trajectories import simulate_motion_comparison


def test_position_integral():
    """Test different position integral gains."""
    print("=" * 80)
    print("POSITION INTEGRAL GAIN TUNING")
    print("=" * 80)
    print("\nProblem: Settling phase has 4.3° RMS error due to steady-state bias")
    print("Solution: Add position integral term (PI outer loop)\n")

    # Baseline: S-curve with Option S1 (no position integral)
    print("Testing configurations with S-curve (jerk=100 rad/s³)...\n")

    configs = [
        {
            "name": "Baseline (P only)",
            "ki_pos": 0.0,
            "ki_vel": 3.5,
            "kff_accel": 0.5,
        },
        {
            "name": "Low PI (ki_pos=0.5)",
            "ki_pos": 0.5,
            "ki_vel": 3.5,
            "kff_accel": 0.5,
        },
        {
            "name": "Medium PI (ki_pos=1.0)",
            "ki_pos": 1.0,
            "ki_vel": 3.5,
            "kff_accel": 0.5,
        },
        {
            "name": "High PI (ki_pos=1.5)",
            "ki_pos": 1.5,
            "ki_vel": 3.5,
            "kff_accel": 0.5,
        },
        {
            "name": "Very High PI (ki_pos=2.0)",
            "ki_pos": 2.0,
            "ki_vel": 3.5,
            "kff_accel": 0.5,
        },
        {
            "name": "Aggressive PI (ki_pos=2.5)",
            "ki_pos": 2.5,
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
            ki_pos=config["ki_pos"],
            kp_vel=3.5,
            ki_vel=config["ki_vel"],
            kd_vel=1.0,
            kff_vel=1.0,
            kff_accel=config["kff_accel"],
            max_jerk=100.0,
        )

        config["metrics"] = metrics
        results.append(config)

        print(f"  RMS:      {metrics['rms_error_deg']:.3f}°")
        print(f"  Max:      {metrics['max_error_deg']:.3f}°")
        print(f"  Overshoot: {metrics['overshoot_percent']:.1f}%")
        print(
            f"  Settling:  {metrics['settling_rms_deg']:.3f}° RMS, {metrics['settling_mean_deg']:.3f}° mean"
        )
        print()

    # Comparison table
    print("=" * 80)
    print("RESULTS")
    print("=" * 80)
    print()
    print(
        f"{'Configuration':<30} {'RMS°':>8} {'Max°':>8} {'OS%':>6} {'Settle°':>9} {'Status':>10}"
    )
    print("-" * 80)

    baseline = results[0]["metrics"]

    for r in results:
        m = r["metrics"]
        improvement = (
            (baseline["rms_error_deg"] - m["rms_error_deg"])
            / baseline["rms_error_deg"]
            * 100
        )

        status = ""
        if m["rms_error_deg"] < 1.0:
            status = "✅ GOAL!"
        elif m["overshoot_percent"] > 10:
            status = "❌ OVERSHOOT"
        elif improvement > 0:
            status = "✓ Better"
        else:
            status = "Worse"

        print(
            f"{r['name']:<30} "
            f"{m['rms_error_deg']:>8.3f} "
            f"{m['max_error_deg']:>8.3f} "
            f"{m['overshoot_percent']:>6.1f} "
            f"{m['settling_rms_deg']:>9.3f} "
            f"{status:>10}"
        )
        if r != results[0]:
            print(f"  → Change: {improvement:+.1f}%")

    # Find best
    print()
    print("=" * 80)
    print("BEST CONFIGURATION")
    print("=" * 80)
    print()

    # Filter for valid configs
    valid = [r for r in results if r["metrics"]["overshoot_percent"] < 10]

    if valid:
        best = min(valid, key=lambda x: x["metrics"]["rms_error_deg"])
        m = best["metrics"]

        print(f"🏆 {best['name']}")
        print()
        print("Final Configuration (Option S2):")
        print(f"  kp_pos = 6.0")
        print(f"  ki_pos = {best['ki_pos']} ← NEW!")
        print(f"  kp_vel = 3.5")
        print(f"  ki_vel = {best['ki_vel']}")
        print(f"  kd_vel = 1.0")
        print(f"  kff_vel = 1.0")
        print(f"  kff_accel = {best['kff_accel']}")
        print(f"  max_jerk = 100.0 rad/s³")
        print()
        print("Performance:")
        print(f"  Overall RMS:      {m['rms_error_deg']:.3f}°")
        print(f"  Accel Phase:      {m['accel_phase_rms_deg']:.3f}°")
        print(f"  Decel Phase:      {m['decel_phase_rms_deg']:.3f}°")
        print(f"  Settling Phase:   {m['settling_rms_deg']:.3f}° (was 4.316°)")
        print(f"  Steady-State:     {abs(m['settling_mean_deg']):.3f}° (was 4.303°)")
        print(f"  Max Error:        {m['max_error_deg']:.3f}°")
        print(f"  Overshoot:        {m['overshoot_percent']:.1f}%")
        print()

        improvement_vs_baseline = (
            (baseline["rms_error_deg"] - m["rms_error_deg"])
            / baseline["rms_error_deg"]
            * 100
        )
        improvement_vs_trap = (3.567 - m["rms_error_deg"]) / 3.567 * 100
        improvement_vs_original = (61.0 - m["rms_error_deg"]) / 61.0 * 100

        print("Improvement Summary:")
        print(f"  vs Option S1 (no pos integral): {improvement_vs_baseline:+.1f}%")
        print(f"  vs Trapezoidal (Option 14):     {improvement_vs_trap:+.1f}%")
        print(f"  vs Original (broken):            {improvement_vs_original:+.1f}%")
        print()

        if m["rms_error_deg"] < 1.0:
            print("=" * 80)
            print("🎯 🎉 TARGET ACHIEVED: RMS < 1°! 🎉 🎯")
            print("=" * 80)
        else:
            gap = m["rms_error_deg"] - 1.0
            print(f"⚠️  Close! Only {gap:.3f}° away from <1° target")
            print(f"   ({gap / m['rms_error_deg'] * 100:.1f}% reduction needed)")

    print()
    print("=" * 80)

    return results


if __name__ == "__main__":
    test_position_integral()
