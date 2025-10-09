#!/usr/bin/env python3
"""
Fix Overshoot Problem - Reduce Velocity Integral Aggressiveness

The system overshoots by 4.3° due to excessive ki_vel=3.5.
Test configurations with reduced integral gain and increased damping.
"""

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parent / "renode" / "tests"))

from compare_trajectories import simulate_motion_comparison


def fix_overshoot():
    """Test configurations to fix overshoot/settling error."""
    print("=" * 80)
    print("FIX OVERSHOOT PROBLEM")
    print("=" * 80)
    print("\nRoot Cause: System overshoots target by 4.3° due to ki_vel=3.5 being too high")
    print("Solutions: Reduce ki_vel OR increase kd_vel OR both\n")

    configs = [
        {
            "name": "Baseline (Overshooting)",
            "ki_vel": 3.5,
            "kd_vel": 1.0,
            "kff_accel": 0.5,
        },
        {
            "name": "Reduce Integral (ki=3.0)",
            "ki_vel": 3.0,
            "kd_vel": 1.0,
            "kff_accel": 0.5,
        },
        {
            "name": "Reduce Integral (ki=2.5)",
            "ki_vel": 2.5,
            "kd_vel": 1.0,
            "kff_accel": 0.5,
        },
        {
            "name": "Reduce Integral (ki=2.0)",
            "ki_vel": 2.0,
            "kd_vel": 1.0,
            "kff_accel": 0.5,
        },
        {
            "name": "More Damping (kd=1.5)",
            "ki_vel": 3.5,
            "kd_vel": 1.5,
            "kff_accel": 0.5,
        },
        {
            "name": "More Damping (kd=2.0)",
            "ki_vel": 3.5,
            "kd_vel": 2.0,
            "kff_accel": 0.5,
        },
        {
            "name": "Balanced (ki=2.5, kd=1.5)",
            "ki_vel": 2.5,
            "kd_vel": 1.5,
            "kff_accel": 0.5,
        },
        {
            "name": "Conservative (ki=2.0, kd=1.5)",
            "ki_vel": 2.0,
            "kd_vel": 1.5,
            "kff_accel": 0.5,
        },
        {
            "name": "Very Damped (ki=2.5, kd=2.0)",
            "ki_vel": 2.5,
            "kd_vel": 2.0,
            "kff_accel": 0.5,
        },
    ]

    results = []

    for config in configs:
        print(f"Testing: {config['name']}...")

        metrics = simulate_motion_comparison(
            trajectory_type="scurve",
            kp_pos=6.0,
            ki_pos=0.0,
            kp_vel=3.5,
            ki_vel=config["ki_vel"],
            kd_vel=config["kd_vel"],
            kff_vel=1.0,
            kff_accel=config["kff_accel"],
            max_jerk=100.0,
        )

        config["metrics"] = metrics
        results.append(config)

        print(f"  RMS:           {metrics['rms_error_deg']:.3f}°")
        print(f"  Settling mean: {metrics['settling_mean_deg']:.3f}° (target: ~0°)")
        print(f"  Overshoot:     {metrics['overshoot_percent']:.1f}%")
        print()

    # Comparison table
    print("=" * 80)
    print("RESULTS")
    print("=" * 80)
    print()
    print(f"{'Configuration':<35} {'RMS°':>8} {'OS%':>6} {'Settle°':>9} {'SS Bias°':>10}")
    print("-" * 80)

    baseline = results[0]["metrics"]

    for r in results:
        m = r["metrics"]

        status = ""
        if m["rms_error_deg"] < 1.0:
            status = " ✅"
        elif m["overshoot_percent"] > 8:
            status = " ⚠️"

        print(
            f"{r['name']:<35} "
            f"{m['rms_error_deg']:>8.3f} "
            f"{m['overshoot_percent']:>6.1f} "
            f"{m['settling_rms_deg']:>9.3f} "
            f"{m['settling_mean_deg']:>10.3f}{status}"
        )

    # Find best
    print()
    print("=" * 80)
    print("BEST CONFIGURATION")
    print("=" * 80)
    print()

    # Prioritize low RMS error
    valid = [r for r in results if r["metrics"]["overshoot_percent"] < 10]
    best = min(valid, key=lambda x: x["metrics"]["rms_error_deg"])
    m = best["metrics"]

    print(f"🏆 {best['name']}")
    print()
    print("Optimized Configuration (Option S3):")
    print(f"  kp_pos = 6.0")
    print(f"  ki_pos = 0.0")
    print(f"  kp_vel = 3.5")
    print(f"  ki_vel = {best['ki_vel']} ← Adjusted from 3.5")
    print(f"  kd_vel = {best['kd_vel']} ← Adjusted from 1.0")
    print(f"  kff_vel = 1.0")
    print(f"  kff_accel = {best['kff_accel']}")
    print(f"  max_jerk = 100.0 rad/s³")
    print()
    print("Performance:")
    print(f"  Overall RMS:      {m['rms_error_deg']:.3f}°")
    print(f"  Accel Phase:      {m['accel_phase_rms_deg']:.3f}°")
    print(f"  Decel Phase:      {m['decel_phase_rms_deg']:.3f}°")
    print(f"  Settling Phase:   {m['settling_rms_deg']:.3f}° (was 4.316°)")
    print(f"  Steady-State Bias: {abs(m['settling_mean_deg']):.3f}° (was 4.303°)")
    print(f"  Max Error:        {m['max_error_deg']:.3f}°")
    print(f"  Overshoot:        {m['overshoot_percent']:.1f}%")
    print()

    improvement_vs_s1 = (2.210 - m["rms_error_deg"]) / 2.210 * 100
    improvement_vs_trap = (3.567 - m["rms_error_deg"]) / 3.567 * 100
    improvement_vs_original = (61.0 - m["rms_error_deg"]) / 61.0 * 100

    print("Improvement Summary:")
    print(f"  vs Option S1 (ki=3.5):           {improvement_vs_s1:+.1f}%")
    print(f"  vs Trapezoidal (Option 14):      {improvement_vs_trap:+.1f}%")
    print(f"  vs Original (broken):             {improvement_vs_original:+.1f}%")
    print()

    if m["rms_error_deg"] < 1.0:
        print("=" * 80)
        print("🎯 🎉 TARGET ACHIEVED: RMS < 1°! 🎉 🎯")
        print("=" * 80)
    else:
        gap = m["rms_error_deg"] - 1.0
        print(f"📊 Gap to <1° target: {gap:.3f}° ({gap / m['rms_error_deg'] * 100:.1f}% of current RMS)")

    print()
    print("=" * 80)

    return results


if __name__ == "__main__":
    fix_overshoot()
