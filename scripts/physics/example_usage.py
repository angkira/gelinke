#!/usr/bin/env python3
"""
Example: How to use motor physics model

Demonstrates the difference between:
1. Kinematic model (WRONG - instant response)
2. Dynamic model (CORRECT - realistic physics)
"""

import os
import numpy as np
import matplotlib.pyplot as plt
from motor_model import MotorSimulator, MotorParameters


def kinematic_simulation(duration=0.5, dt=0.0001):
    """OLD WAY: Kinematic approximation (no inertia)."""
    n_samples = int(duration / dt)

    position = 0.0
    velocity = 0.0

    time = np.zeros(n_samples)
    pos_arr = np.zeros(n_samples)
    vel_arr = np.zeros(n_samples)

    target_pos = 1.57  # 90 degrees
    kp = 5.0
    kd = 2.0

    for i in range(n_samples):
        t = i * dt
        time[i] = t

        # Simple PD controller
        error = target_pos - position
        accel = kp * error - kd * velocity

        # KINEMATIC UPDATE (WRONG!)
        velocity += accel * dt  # ← Velocity changes instantly!
        position += velocity * dt

        pos_arr[i] = position
        vel_arr[i] = velocity

    return time, pos_arr, vel_arr


def dynamic_simulation(duration=0.5, dt=0.0001):
    """NEW WAY: Dynamic model (with inertia)."""
    params = MotorParameters(
        J=0.001,  # kg·m² - Rotor inertia
        kt=0.15,  # Nm/A - Torque constant
        b=0.0005,  # Nm·s/rad - Damping
    )

    sim = MotorSimulator(params, sample_rate=1.0 / dt)

    target_pos = 1.57  # 90 degrees
    kp_current = 5.0  # Current (A) per rad error
    kd_current = 2.0  # Current (A) per rad/s velocity

    def controller(t, state):
        """PD controller that outputs i_q current."""
        error = target_pos - state["position"]
        i_q = kp_current * error - kd_current * state["velocity"]
        return i_q

    result = sim.simulate_trajectory(controller, duration)

    return result["time"], result["position"], result["velocity"]


def main():
    """Compare kinematic vs dynamic models."""
    print("=" * 70)
    print("Motor Physics Model - Comparison Demo")
    print("=" * 70)

    # Run both simulations
    print("\nRunning kinematic simulation (OLD)...")
    t_kin, pos_kin, vel_kin = kinematic_simulation()

    print("Running dynamic simulation (NEW)...")
    t_dyn, pos_dyn, vel_dyn = dynamic_simulation()

    # Calculate metrics
    target = 1.57

    # Kinematic metrics
    error_kin = pos_kin - target
    rms_kin = np.sqrt(np.mean(error_kin**2))
    overshoot_kin = (
        (np.max(pos_kin) - target) / target * 100 if np.max(pos_kin) > target else 0.0
    )

    # Dynamic metrics
    error_dyn = pos_dyn - target
    rms_dyn = np.sqrt(np.mean(error_dyn**2))
    overshoot_dyn = (
        (np.max(pos_dyn) - target) / target * 100 if np.max(pos_dyn) > target else 0.0
    )

    print("\n" + "=" * 70)
    print("RESULTS COMPARISON")
    print("=" * 70)
    print(f"\n{'Metric':<30} {'Kinematic':<15} {'Dynamic':<15}")
    print("-" * 70)
    print(f"{'Final position (rad)':<30} {pos_kin[-1]:<15.3f} {pos_dyn[-1]:<15.3f}")
    print(f"{'Final velocity (rad/s)':<30} {vel_kin[-1]:<15.3f} {vel_dyn[-1]:<15.3f}")
    print(f"{'RMS error (rad)':<30} {rms_kin:<15.3f} {rms_dyn:<15.3f}")
    print(
        f"{'RMS error (degrees)':<30} {np.rad2deg(rms_kin):<15.1f} {np.rad2deg(rms_dyn):<15.1f}"
    )
    print(f"{'Overshoot (%)':<30} {overshoot_kin:<15.1f} {overshoot_dyn:<15.1f}")
    print(
        f"{'Max velocity (rad/s)':<30} {np.max(vel_kin):<15.2f} {np.max(vel_dyn):<15.2f}"
    )

    print("\n" + "=" * 70)
    print("ANALYSIS")
    print("=" * 70)

    if rms_kin < rms_dyn:
        print("⚠️  WARNING: Kinematic model shows UNREALISTICALLY good performance!")
        print("    This is because it has no inertia - motor responds instantly.")
        print("    Real hardware will perform worse.")
    else:
        print("✅ Dynamic model is more realistic (includes inertia).")
        print("   Performance matches what you'll see on hardware.")

    # Plot comparison
    fig, axes = plt.subplots(2, 1, figsize=(10, 8))

    # Position
    axes[0].plot(t_kin, pos_kin, "r-", label="Kinematic (no inertia)", linewidth=2)
    axes[0].plot(t_dyn, pos_dyn, "b-", label="Dynamic (with inertia)", linewidth=2)
    axes[0].axhline(target, color="k", linestyle="--", alpha=0.3, label="Target")
    axes[0].set_ylabel("Position (rad)")
    axes[0].legend()
    axes[0].grid(True, alpha=0.3)
    axes[0].set_title("Position Response Comparison")

    # Velocity
    axes[1].plot(t_kin, vel_kin, "r-", label="Kinematic", linewidth=2)
    axes[1].plot(t_dyn, vel_dyn, "b-", label="Dynamic", linewidth=2)
    axes[1].set_xlabel("Time (s)")
    axes[1].set_ylabel("Velocity (rad/s)")
    axes[1].legend()
    axes[1].grid(True, alpha=0.3)
    axes[1].set_title("Velocity Response Comparison")

    plt.tight_layout()
    # Determine output path relative to script location
    script_dir = os.path.dirname(os.path.abspath(__file__))
    project_root = os.path.dirname(os.path.dirname(script_dir))
    output_file = os.path.join(project_root, "demo_results", "kinematic_vs_dynamic.png")
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"\n📊 Plot saved: {output_file}")

    print("\n" + "=" * 70)
    print("CONCLUSION")
    print("=" * 70)
    print("\n✅ Always use MotorDynamics for realistic simulation!")
    print("❌ Avoid kinematic models (v += a*dt) - they give false hope!")
    print("\n" + "=" * 70)


if __name__ == "__main__":
    main()
