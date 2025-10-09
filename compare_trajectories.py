#!/usr/bin/env python3
"""
Compare Trapezoidal vs S-curve Trajectory Performance

This script systematically compares the tracking performance of
trapezoidal and S-curve trajectories using the optimized Option 14 controller.
"""

import sys
import numpy as np
from pathlib import Path
from typing import Dict, Any

# Add renode/tests to path
sys.path.insert(0, str(Path(__file__).parent / "renode" / "tests"))

from demo_visualization import (
    PIDController,
    calculate_control_metrics,
    generate_scurve_trajectory,
)


def simulate_motion_comparison(
    trajectory_type: str,
    kp_pos: float = 6.0,
    ki_pos: float = 0.0,
    kp_vel: float = 3.5,
    ki_vel: float = 1.5,
    kd_vel: float = 1.0,
    kff_vel: float = 1.0,
    kff_accel: float = 0.3,
    kff_friction: float = 0.0,
    max_jerk: float = 50.0,
) -> Dict[str, Any]:
    """Run simulation with specified trajectory type and return metrics.

    Args:
        trajectory_type: "trapezoidal" or "scurve"
        kp_pos: Position proportional gain
        ki_pos: Position integral gain
        kp_vel: Velocity proportional gain
        ki_vel: Velocity integral gain
        kd_vel: Velocity derivative gain
        kff_vel: Velocity feedforward gain
        kff_accel: Acceleration feedforward gain
        max_jerk: Maximum jerk for S-curve (rad/s³)

    Returns:
        Dictionary with simulation results and metrics
    """
    # Motion parameters
    target = 1.57  # 90 degrees
    max_vel = 2.0  # rad/s
    max_accel = 5.0  # rad/s²
    dt = 0.0001  # 10 kHz simulation

    # Calculate motion phases for trapezoidal
    t_accel = max_vel / max_accel
    t_coast = 0.0
    t_decel = t_accel

    x_accel = 0.5 * max_accel * t_accel**2
    if 2 * x_accel < target:
        t_coast = (target - 2 * x_accel) / max_vel
    else:
        t_accel = np.sqrt(target / max_accel)
        t_decel = t_accel
        max_vel = max_accel * t_accel

    duration = t_accel + t_coast + t_decel + 0.2  # Add settling time
    n_samples = int(duration / dt)

    # Initialize state
    position = 0.0
    velocity = 0.0
    pos_integral = 0.0  # Position integral term

    # Initialize controller
    vel_controller = PIDController(
        kp=kp_vel,
        ki=ki_vel,
        kd=kd_vel,
        max_integral=max_vel,
        max_output=max_accel,
    )

    # Storage for analysis
    position_arr = []
    velocity_arr = []
    target_pos_arr = []
    target_vel_arr = []
    target_accel_arr = []
    pos_error_arr = []
    vel_error_arr = []
    time_arr = []

    # Simulation loop
    for i in range(n_samples):
        t = i * dt

        # Generate trajectory
        if trajectory_type == "scurve":
            target_pos, target_vel, target_accel, _ = generate_scurve_trajectory(
                t, target, max_vel, max_accel, max_jerk
            )
        else:
            # Trapezoidal
            if t < t_accel:
                target_vel = max_accel * t
                target_pos = 0.5 * max_accel * t**2
                target_accel = max_accel
            elif t < t_accel + t_coast:
                target_vel = max_vel
                target_pos = 0.5 * max_accel * t_accel**2 + max_vel * (t - t_accel)
                target_accel = 0.0
            elif t < t_accel + t_coast + t_decel:
                t_dec = t - t_accel - t_coast
                target_vel = max_vel - max_accel * t_dec
                target_pos = (
                    0.5 * max_accel * t_accel**2
                    + max_vel * t_coast
                    + max_vel * t_dec
                    - 0.5 * max_accel * t_dec**2
                )
                target_accel = -max_accel
            else:
                target_vel = 0.0
                target_pos = target
                target_accel = 0.0

        # Controller (cascade with feedforward)
        pos_error = target_pos - position

        # Outer loop (position -> velocity) with PI control
        pos_integral += pos_error * dt
        pos_integral = np.clip(pos_integral, -0.2, 0.2)  # Anti-windup

        target_vel_from_pos = kp_pos * pos_error + ki_pos * pos_integral
        target_vel_from_pos = np.clip(target_vel_from_pos, -max_vel, max_vel)

        # Combine with feedforward
        target_vel_combined = kff_vel * target_vel + target_vel_from_pos

        # Inner loop (velocity -> acceleration)
        vel_error = target_vel_combined - velocity
        accel_fb = vel_controller.update(vel_error, dt, feedforward=0.0)

        # Add acceleration feedforward + friction compensation
        accel_ff = kff_accel * target_accel + kff_friction * velocity
        accel = accel_fb + accel_ff

        # Apply saturation
        accel = np.clip(accel, -max_accel, max_accel)
        velocity += accel * dt
        velocity = np.clip(velocity, -max_vel, max_vel)
        position += velocity * dt

        # Store data
        position_arr.append(position)
        velocity_arr.append(velocity)
        target_pos_arr.append(target_pos)
        target_vel_arr.append(target_vel)
        target_accel_arr.append(target_accel)
        pos_error_arr.append(pos_error)
        vel_error_arr.append(vel_error)
        time_arr.append(t)

    # Convert to numpy arrays
    position_arr = np.array(position_arr)
    velocity_arr = np.array(velocity_arr)
    target_pos_arr = np.array(target_pos_arr)
    target_vel_arr = np.array(target_vel_arr)
    target_accel_arr = np.array(target_accel_arr)
    pos_error_arr = np.array(pos_error_arr)
    vel_error_arr = np.array(vel_error_arr)
    time_arr = np.array(time_arr)

    # Calculate tracking error (position - target_position at each timestep)
    tracking_error_arr = position_arr - target_pos_arr
    rms_tracking_error = np.rad2deg(np.sqrt(np.mean(tracking_error_arr**2)))
    max_tracking_error = np.rad2deg(np.max(np.abs(tracking_error_arr)))

    # Calculate metrics using final target for overshoot detection
    metrics = calculate_control_metrics(position_arr, target, velocity_arr, max_vel, dt)

    # Override RMS and max error with actual tracking errors
    metrics["rms_error_deg"] = rms_tracking_error
    metrics["max_error_deg"] = max_tracking_error

    # Additional phase-specific analysis
    accel_phase_idx = np.where(time_arr < t_accel)[0]
    decel_phase_start = t_accel + t_coast
    decel_phase_idx = np.where((time_arr >= decel_phase_start) &
                                (time_arr < decel_phase_start + t_decel))[0]
    settling_phase_idx = np.where(time_arr >= decel_phase_start + t_decel)[0]

    # Phase metrics (using tracking error = actual - target)
    if len(accel_phase_idx) > 0:
        accel_phase_rms = np.rad2deg(
            np.sqrt(np.mean(tracking_error_arr[accel_phase_idx] ** 2))
        )
        accel_phase_max = np.rad2deg(np.max(np.abs(tracking_error_arr[accel_phase_idx])))
        accel_vel_rms = np.sqrt(np.mean(vel_error_arr[accel_phase_idx] ** 2))
    else:
        accel_phase_rms = 0.0
        accel_phase_max = 0.0
        accel_vel_rms = 0.0

    if len(decel_phase_idx) > 0:
        decel_phase_rms = np.rad2deg(
            np.sqrt(np.mean(tracking_error_arr[decel_phase_idx] ** 2))
        )
        decel_phase_max = np.rad2deg(np.max(np.abs(tracking_error_arr[decel_phase_idx])))
    else:
        decel_phase_rms = 0.0
        decel_phase_max = 0.0

    if len(settling_phase_idx) > 0:
        settling_mean = np.rad2deg(np.mean(tracking_error_arr[settling_phase_idx]))
        settling_rms = np.rad2deg(
            np.sqrt(np.mean(tracking_error_arr[settling_phase_idx] ** 2))
        )
        settling_max = np.rad2deg(np.max(np.abs(tracking_error_arr[settling_phase_idx])))
    else:
        settling_mean = 0.0
        settling_rms = 0.0
        settling_max = 0.0

    # Add phase metrics
    metrics.update({
        "accel_phase_rms_deg": accel_phase_rms,
        "accel_phase_max_deg": accel_phase_max,
        "accel_vel_rms": accel_vel_rms,
        "decel_phase_rms_deg": decel_phase_rms,
        "decel_phase_max_deg": decel_phase_max,
        "settling_mean_deg": settling_mean,
        "settling_rms_deg": settling_rms,
        "settling_max_deg": settling_max,
        "trajectory_type": trajectory_type,
        "max_jerk": max_jerk if trajectory_type == "scurve" else float("inf"),
    })

    return metrics


def compare_trajectories():
    """Compare trapezoidal vs S-curve trajectories."""
    print("=" * 80)
    print("TRAJECTORY COMPARISON: TRAPEZOIDAL VS S-CURVE")
    print("=" * 80)
    print("\nUsing Option 14 Controller:")
    print("  kp_pos=6.0, kp_vel=3.5, ki_vel=1.5, kd_vel=1.0")
    print("  kff_vel=1.0, kff_accel=0.3")
    print()

    # Test different jerk limits for S-curve
    configs = [
        {"name": "Trapezoidal (Baseline)", "type": "trapezoidal", "jerk": None},
        {"name": "S-curve (Jerk: 50 rad/s³)", "type": "scurve", "jerk": 50.0},
        {"name": "S-curve (Jerk: 80 rad/s³)", "type": "scurve", "jerk": 80.0},
        {"name": "S-curve (Jerk: 100 rad/s³)", "type": "scurve", "jerk": 100.0},
        {"name": "S-curve (Jerk: 120 rad/s³)", "type": "scurve", "jerk": 120.0},
        {"name": "S-curve (Jerk: 150 rad/s³)", "type": "scurve", "jerk": 150.0},
        {"name": "S-curve (Jerk: 200 rad/s³)", "type": "scurve", "jerk": 200.0},
    ]

    results = []

    for config in configs:
        print(f"Testing: {config['name']}...")

        if config["type"] == "trapezoidal":
            metrics = simulate_motion_comparison(
                trajectory_type="trapezoidal",
                max_jerk=50.0,  # Not used
            )
        else:
            metrics = simulate_motion_comparison(
                trajectory_type="scurve",
                max_jerk=config["jerk"],
            )

        results.append(metrics)
        print(f"  ✓ RMS error: {metrics['rms_error_deg']:.3f}°")
        print(f"  ✓ Max error: {metrics['max_error_deg']:.3f}°")
        print(f"  ✓ Overshoot: {metrics['overshoot_percent']:.1f}%")
        print()

    # Comparison table
    print("=" * 80)
    print("COMPARISON RESULTS")
    print("=" * 80)
    print()
    print(f"{'Configuration':<40} {'RMS°':>8} {'Max°':>8} {'OS%':>6} {'Accel°':>8} {'Decel°':>8}")
    print("-" * 80)

    baseline = results[0]

    for r in results:
        rms_improvement = ""
        if r != baseline and baseline["rms_error_deg"] > 0:
            improvement = (baseline["rms_error_deg"] - r["rms_error_deg"]) / baseline["rms_error_deg"] * 100
            if improvement > 0:
                rms_improvement = f" (-{improvement:.1f}%)"
            else:
                rms_improvement = f" (+{abs(improvement):.1f}%)"

        name = r["trajectory_type"].upper()
        if r["trajectory_type"] == "scurve":
            name += f" (J={r['max_jerk']:.0f})"

        print(
            f"{name:<40} "
            f"{r['rms_error_deg']:>8.3f} "
            f"{r['max_error_deg']:>8.3f} "
            f"{r['overshoot_percent']:>6.1f} "
            f"{r['accel_phase_rms_deg']:>8.3f} "
            f"{r['decel_phase_rms_deg']:>8.3f}"
        )
        if rms_improvement:
            print(f"  → Improvement: {rms_improvement}")

    # Best configuration
    print()
    print("=" * 80)
    print("ANALYSIS")
    print("=" * 80)
    print()

    best = min(results, key=lambda x: x["rms_error_deg"])

    print(f"🏆 Best Configuration: {best['trajectory_type'].upper()}")
    if best["trajectory_type"] == "scurve":
        print(f"   Max Jerk: {best['max_jerk']:.0f} rad/s³")
    print()

    print(f"Performance vs Baseline (Trapezoidal):")
    print(f"  RMS Error:     {baseline['rms_error_deg']:.3f}° → {best['rms_error_deg']:.3f}° "
          f"({(baseline['rms_error_deg'] - best['rms_error_deg']) / baseline['rms_error_deg'] * 100:.1f}% improvement)")
    print(f"  Max Error:     {baseline['max_error_deg']:.3f}° → {best['max_error_deg']:.3f}° "
          f"({(baseline['max_error_deg'] - best['max_error_deg']) / baseline['max_error_deg'] * 100:.1f}% improvement)")
    print(f"  Overshoot:     {baseline['overshoot_percent']:.1f}% → {best['overshoot_percent']:.1f}%")
    print()

    print("Phase-by-Phase Breakdown (Best Configuration):")
    print(f"  Acceleration Phase:  RMS = {best['accel_phase_rms_deg']:.3f}°, Max = {best['accel_phase_max_deg']:.3f}°")
    print(f"  Deceleration Phase:  RMS = {best['decel_phase_rms_deg']:.3f}°, Max = {best['decel_phase_max_deg']:.3f}°")
    print(f"  Settling Phase:      RMS = {best['settling_rms_deg']:.3f}°, Mean = {best['settling_mean_deg']:.3f}°")
    print()

    # Goal assessment
    print("Goal Achievement:")
    print(f"  Target: RMS < 1° → Current: {best['rms_error_deg']:.3f}° ", end="")
    if best['rms_error_deg'] < 1.0:
        print("✅ ACHIEVED!")
    else:
        print(f"❌ Not yet (need {best['rms_error_deg'] - 1.0:.3f}° more)")

    print()
    print("=" * 80)
    print("RECOMMENDATION")
    print("=" * 80)
    print()

    if best["trajectory_type"] == "scurve":
        print(f"✅ Use S-curve trajectory with max_jerk = {best['max_jerk']:.0f} rad/s³")
        print(f"   RMS improvement: {(baseline['rms_error_deg'] - best['rms_error_deg']) / baseline['rms_error_deg'] * 100:.1f}%")
        print(f"   Expected RMS error: {best['rms_error_deg']:.3f}°")
    else:
        print("⚠️  S-curve did not improve performance significantly.")
        print("   Consider other approaches (model-based feedforward, MPC)")

    print()
    print("=" * 80)

    return results


if __name__ == "__main__":
    compare_trajectories()
