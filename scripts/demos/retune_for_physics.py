#!/usr/bin/env python3
"""
Re-tune controller gains for realistic motor physics.

This script re-runs controller tuning with MotorDynamics instead of kinematic model.
"""

import sys
import numpy as np
from pathlib import Path

# Add paths
sys.path.insert(0, str(Path(__file__).parent.parent / "physics"))
sys.path.insert(0, str(Path(__file__).parent))

from motor_model import MotorDynamics, MotorParameters
from demo_visualization import PIDController, calculate_control_metrics


def test_controller_config(config: dict) -> dict:
    """Test a controller configuration with realistic physics."""
    # Motion parameters
    target = 1.57  # 90 degrees
    max_vel = 2.0  # rad/s
    max_accel = 50.0  # rad/s² (increased for realistic physics)
    dt = 0.0001

    # Calculate motion phases
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

    duration = t_accel + t_coast + t_decel + 0.2
    n_samples = int(duration / dt)

    # Initialize motor with realistic physics (low friction for tuning)
    motor_params = MotorParameters(
        J=0.001,  # kg·m²
        kt=0.15,  # Nm/A
        b=0.0005,  # Nm·s/rad
        tau_coulomb=0.002,  # Reduced friction for tuning
        tau_stribeck=0.001,
        v_stribeck=0.1,
        b_viscous=0.0001,
    )
    motor = MotorDynamics(motor_params)
    motor.reset(position=0.0, velocity=0.0)

    # Storage
    position_arr = []
    velocity_arr = []

    # Initialize controller
    if config["use_improved"]:
        vel_controller = PIDController(
            kp=config["kp_vel"],
            ki=config["ki_vel"],
            kd=config["kd_vel"],
            max_integral=max_vel,
            max_output=max_accel,
        )

    for i in range(n_samples):
        t = i * dt

        # Motion profile
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

        # Controller
        pos_error = target_pos - motor.position

        if config["use_improved"]:
            # Cascade control
            target_vel_from_pos = config["kp_pos"] * pos_error
            target_vel_from_pos = np.clip(target_vel_from_pos, -max_vel, max_vel)

            kff_vel = config.get("kff_vel", 1.0)
            kff_accel = config.get("kff_accel", 0.0)

            target_vel_combined = kff_vel * target_vel + target_vel_from_pos

            vel_error = target_vel_combined - motor.velocity
            accel_fb = vel_controller.update(vel_error, dt, feedforward=0.0)

            accel_ff = kff_accel * target_accel
            accel = accel_fb + accel_ff
            accel = np.clip(accel, -max_accel, max_accel)

            # Convert to current
            desired_torque = motor_params.J * accel
            i_q = desired_torque / motor_params.kt
            i_q = np.clip(i_q, -10.0, 10.0)
        else:
            # Original broken controller
            vel_error = target_vel - motor.velocity
            kp_pos_orig = 20.0
            kp_vel_orig = 0.5

            accel = kp_pos_orig * pos_error + kp_vel_orig * vel_error
            desired_torque = motor_params.J * accel
            i_q = desired_torque / motor_params.kt
            i_q = np.clip(i_q, -10.0, 10.0)

        # Update motor dynamics
        state = motor.update(i_q, external_load=0.0, dt=dt)

        position_arr.append(state["position"])
        velocity_arr.append(state["velocity"])

    # Calculate metrics
    position_arr = np.array(position_arr)
    velocity_arr = np.array(velocity_arr)

    metrics = calculate_control_metrics(
        position_arr,
        target,  # Scalar target
        velocity_arr,
        max_vel,
        dt,
    )

    return metrics


def main():
    """Run controller gain tuning with realistic physics."""
    print("=" * 80)
    print("🔧 CONTROLLER GAIN TUNING (WITH REALISTIC PHYSICS)")
    print("=" * 80)
    print()
    print("Using MotorDynamics for accurate simulation!")
    print()

    # Test configurations
    test_configs = [
        {
            "name": "Original (Broken)",
            "use_improved": False,
            "kp_pos": 20.0,
            "kp_vel": 0.5,
            "ki_vel": 2.0,
            "kd_vel": 0.0,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 1: Low gains (10x baseline)",
            "use_improved": True,
            "kp_pos": 100.0,
            "kp_vel": 20.0,
            "ki_vel": 10.0,
            "kd_vel": 2.0,
            "kff_vel": 1.0,
            "kff_accel": 0.5,
        },
        {
            "name": "Option 2: Medium gains (20x baseline)",
            "use_improved": True,
            "kp_pos": 200.0,
            "kp_vel": 40.0,
            "ki_vel": 20.0,
            "kd_vel": 4.0,
            "kff_vel": 1.0,
            "kff_accel": 0.7,
        },
        {
            "name": "Option 3: High gains (30x baseline)",
            "use_improved": True,
            "kp_pos": 300.0,
            "kp_vel": 60.0,
            "ki_vel": 30.0,
            "kd_vel": 6.0,
            "kff_vel": 1.0,
            "kff_accel": 0.8,
        },
        {
            "name": "Option 4: Very high gains (50x baseline)",
            "use_improved": True,
            "kp_pos": 500.0,
            "kp_vel": 100.0,
            "ki_vel": 50.0,
            "kd_vel": 10.0,
            "kff_vel": 1.0,
            "kff_accel": 0.9,
        },
        {
            "name": "Option 5: Extreme gains (100x baseline)",
            "use_improved": True,
            "kp_pos": 1000.0,
            "kp_vel": 200.0,
            "ki_vel": 100.0,
            "kd_vel": 20.0,
            "kff_vel": 1.0,
            "kff_accel": 1.0,
        },
    ]

    results = []

    for config in test_configs:
        print(f"\n📊 Testing: {config['name']}")
        kff_v = config.get("kff_vel", 1.0)
        kff_a = config.get("kff_accel", 0.0)
        print(
            f"   Gains: kp_pos={config['kp_pos']}, kp_vel={config['kp_vel']}, "
            f"ki_vel={config['ki_vel']}, kd_vel={config['kd_vel']}"
        )
        if kff_a > 0:
            print(f"   Feedforward: kff_vel={kff_v}, kff_accel={kff_a}")

        metrics = test_controller_config(config)

        # Calculate damping ratio and natural frequency
        omega_n = np.sqrt(config["kp_pos"])
        zeta = config["kp_vel"] / (2 * omega_n) if omega_n > 0 else 0

        print(f"   ωn={omega_n:.2f} rad/s, ζ={zeta:.3f}")
        print(f"   Overshoot: {metrics['overshoot_percent']:.1f}%")
        print(f"   Max velocity: {metrics['max_velocity']:.2f} rad/s (violation: {metrics['velocity_violation_percent']:.1f}%)")
        print(f"   RMS error: {metrics['rms_error_deg']:.3f}°")
        if metrics.get('settling_time') is not None:
            print(f"   Settling time: {metrics['settling_time']:.3f}s")
        else:
            print(f"   Settling time: None")

        # Warnings
        if metrics["overshoot_percent"] > 10.0:
            print(f"   ❌ Overshoot too high ({metrics['overshoot_percent']:.1f}%)")
        if metrics["velocity_violation_percent"] > 0.0:
            print(f"   ❌ Velocity violation ({metrics['velocity_violation_percent']:.1f}%)")
        if metrics["rms_error_deg"] > 10.0:
            print(f"   ⚠️  RMS error high ({metrics['rms_error_deg']:.3f}°)")
        elif metrics["rms_error_deg"] < 1.0:
            print(f"   ✅ Excellent RMS error ({metrics['rms_error_deg']:.3f}°)")

        results.append({**config, "metrics": metrics, "omega_n": omega_n, "zeta": zeta})

    # Print comparison table
    print("\n" + "=" * 80)
    print("📊 COMPARISON TABLE")
    print("=" * 80)
    print(f"{'Config':<40} {'ζ':>6} {'OS%':>6} {'Vel':>6} {'RMS°':>7} {'Settle':>7}")
    print("-" * 80)

    for r in results:
        m = r["metrics"]
        settle_str = f"{m['settling_time']:.2f}s" if m.get('settling_time') else "N/A"
        print(
            f"{r['name']:<40} {r['zeta']:>6.3f} {m['overshoot_percent']:>6.1f} "
            f"{m['max_velocity']:>6.2f} {m['rms_error_deg']:>7.3f} {settle_str:>7}"
        )

    # Find best configuration
    valid_results = [r for r in results if r["metrics"]["velocity_violation_percent"] < 1.0]
    if valid_results:
        best = min(valid_results, key=lambda r: r["metrics"]["rms_error_deg"])
    else:
        best = min(results, key=lambda r: r["metrics"]["rms_error_deg"])

    print("\n" + "=" * 80)
    print("🏆 BEST CONFIGURATION")
    print("=" * 80)
    print(f"\nChosen: {best['name']}")
    print(f"  Gains: kp_pos={best['kp_pos']}, kp_vel={best['kp_vel']}, ki_vel={best['ki_vel']}, kd_vel={best['kd_vel']}")
    print(f"  Feedforward: kff_vel={best.get('kff_vel', 1.0)}, kff_accel={best.get('kff_accel', 0.0)}")
    print(f"  Overshoot: {best['metrics']['overshoot_percent']:.1f}%")
    print(f"  Max velocity: {best['metrics']['max_velocity']:.2f} rad/s")
    print(f"  RMS error: {best['metrics']['rms_error_deg']:.3f}°")
    print(f"  Damping ratio: ζ = {best['zeta']:.3f}")

    if best['metrics']['rms_error_deg'] < 1.0:
        print("\n✅ EXCELLENT! RMS error < 1° achieved!")
    elif best['metrics']['rms_error_deg'] < 5.0:
        print("\n✅ GOOD! RMS error < 5° achieved!")
    elif best['metrics']['rms_error_deg'] < 10.0:
        print("\n⚠️  Acceptable. RMS error < 10°")
    else:
        print("\n❌ Poor performance. Consider more aggressive tuning.")

    print("\n" + "=" * 80)

    return best


if __name__ == "__main__":
    best_config = main()

