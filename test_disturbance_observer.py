#!/usr/bin/env python3
"""Test and compare Disturbance Observer vs Baseline Subtraction."""

import sys
import numpy as np
import matplotlib.pyplot as plt
from pathlib import Path

# Add demo_visualization module from scripts/demos
sys.path.insert(0, str(Path(__file__).parent / "scripts" / "demos"))
from demo_visualization import DisturbanceObserver

# Add physics model
sys.path.insert(0, str(Path(__file__).parent / "scripts" / "physics"))
from motor_model import MotorDynamics, MotorParameters, FrictionModel as PhysicsFrictionModel


def test_observer_vs_baseline():
    """Compare disturbance observer with baseline subtraction."""

    print("=" * 80)
    print("🔬 Disturbance Observer vs Baseline Subtraction Comparison")
    print("=" * 80)
    print()

    # Simulation parameters
    dt = 0.0001  # 10 kHz
    duration = 1.0
    n_samples = int(duration / dt)

    # Motor parameters
    motor_params = MotorParameters(
        J=0.001,  # kg·m² - Inertia
        b=0.0005,  # Nm·s/rad - Damping
        kt=0.15,  # Nm/A - Torque constant
        tau_coulomb=0.02,
        tau_stribeck=0.01,
        v_stribeck=0.1,
        b_viscous=0.001,
    )
    motor = MotorDynamics(motor_params)
    motor.reset(position=0.0, velocity=0.0)

    # Initialize observer (uses demo_visualization's FrictionModel and DisturbanceObserver)
    from demo_visualization import FrictionModel as DemoFrictionModel
    friction_model_demo = DemoFrictionModel(
        tau_coulomb=0.02,
        b_viscous=0.001,
        v_stribeck=0.1,
        tau_stribeck=0.01,
    )
    observer = DisturbanceObserver(
        J=motor_params.J,
        b=motor_params.b,
        kt=motor_params.kt,
        alpha=0.05,
        friction_model=friction_model_demo,
        compensate_friction=True,
    )

    # Storage
    time_arr = []
    position_arr = []
    velocity_arr = []
    external_load_arr = []
    observer_estimate_arr = []
    baseline_estimate_arr = []

    # Baseline learning
    i_q_baseline = 0.0
    baseline_samples = []
    baseline_learned = False

    # Simple position controller (outputs desired acceleration)
    kp = 5.0
    kd = 2.0

    temperature = 25.0

    for i in range(n_samples):
        t = i * dt

        # Target trajectory with motion
        if t < 0.2:
            # Hold at zero
            target_pos = 0.0
            target_vel = 0.0
            external_load = 0.0
        elif t < 0.4:
            # Move to 1 rad
            t_rel = t - 0.2
            target_pos = 0.5 * 5.0 * min(t_rel, 0.2) ** 2  # Accelerate
            target_vel = 5.0 * min(t_rel, 0.2)
            external_load = 0.0
        elif t < 0.6:
            # Hold at 1 rad with external load
            target_pos = 1.0
            target_vel = 0.0
            external_load = 0.3  # 0.3 Nm load applied
        elif t < 0.8:
            # Move back to zero
            t_rel = t - 0.6
            target_pos = 1.0 - 0.5 * 5.0 * min(t_rel, 0.2) ** 2
            target_vel = -5.0 * min(t_rel, 0.2)
            external_load = 0.2  # Load during motion
        else:
            # Hold at zero
            target_pos = 0.0
            target_vel = 0.0
            external_load = 0.0

        # Controller (outputs desired acceleration)
        pos_error = target_pos - motor.position
        accel = kp * pos_error - kd * motor.velocity

        # Convert desired acceleration to motor current
        # Controller wants: τ_desired = J * α
        # Motor needs: i_q = τ_desired / kt
        desired_torque = motor_params.J * accel
        i_q = desired_torque / motor_params.kt

        # Update motor dynamics with realistic physics
        state = motor.update(i_q, external_load=external_load, dt=dt, temperature=temperature)
        position = state["position"]
        velocity = state["velocity"]

        # ===== DISTURBANCE OBSERVER =====
        load_observer = observer.update(velocity, i_q, dt, temperature)

        # ===== BASELINE SUBTRACTION =====
        if not baseline_learned:
            # Learn baseline during first 0.15s
            if t < 0.15:
                baseline_samples.append(i_q)
            else:
                if baseline_samples:
                    i_q_baseline = np.mean(baseline_samples)
                baseline_learned = True

        if baseline_learned:
            i_q_external = i_q - i_q_baseline
            load_baseline_raw = kt * i_q_external

            # Low-pass filter (same as observer)
            if i == 0 or not hasattr(test_observer_vs_baseline, "load_baseline"):
                test_observer_vs_baseline.load_baseline = 0.0

            alpha = 0.05
            load_baseline = (
                alpha * load_baseline_raw
                + (1 - alpha) * test_observer_vs_baseline.load_baseline
            )
            test_observer_vs_baseline.load_baseline = load_baseline
        else:
            load_baseline = 0.0

        # Store data
        if i % 10 == 0:  # 1 kHz logging
            time_arr.append(t)
            position_arr.append(position)
            velocity_arr.append(velocity)
            external_load_arr.append(external_load)
            observer_estimate_arr.append(load_observer)
            baseline_estimate_arr.append(load_baseline)

    # Convert to numpy arrays
    time_arr = np.array(time_arr)
    position_arr = np.array(position_arr)
    velocity_arr = np.array(velocity_arr)
    external_load_arr = np.array(external_load_arr)
    observer_estimate_arr = np.array(observer_estimate_arr)
    baseline_estimate_arr = np.array(baseline_estimate_arr)

    # Calculate errors
    observer_error = observer_estimate_arr - external_load_arr
    baseline_error = baseline_estimate_arr - external_load_arr

    observer_rmse = np.sqrt(np.mean(observer_error**2))
    baseline_rmse = np.sqrt(np.mean(baseline_error**2))

    print("📊 Performance Comparison:")
    print(f"  Disturbance Observer RMSE: {observer_rmse:.4f} Nm")
    print(f"  Baseline Subtraction RMSE: {baseline_rmse:.4f} Nm")
    print(
        f"  Improvement: {(baseline_rmse - observer_rmse) / baseline_rmse * 100:.1f}%"
    )
    print()

    # Analyze different phases
    print("📈 Phase Analysis:")
    print()

    phases = [
        ("Hold (0-0.2s)", 0.0, 0.2),
        ("Motion (0.2-0.4s)", 0.2, 0.4),
        ("Hold + Load (0.4-0.6s)", 0.4, 0.6),
        ("Motion + Load (0.6-0.8s)", 0.6, 0.8),
        ("Hold (0.8-1.0s)", 0.8, 1.0),
    ]

    for phase_name, t_start, t_end in phases:
        mask = (time_arr >= t_start) & (time_arr < t_end)
        if np.any(mask):
            obs_phase = observer_estimate_arr[mask]
            base_phase = baseline_estimate_arr[mask]
            true_phase = external_load_arr[mask]

            obs_err = np.sqrt(np.mean((obs_phase - true_phase) ** 2))
            base_err = np.sqrt(np.mean((base_phase - true_phase) ** 2))

            print(f"  {phase_name}:")
            print(f"    Observer RMSE: {obs_err:.4f} Nm")
            print(f"    Baseline RMSE: {base_err:.4f} Nm")
            if base_err > 0:
                improvement = (base_err - obs_err) / base_err * 100
                symbol = "✅" if improvement > 0 else "⚠️"
                print(f"    {symbol} Improvement: {improvement:+.1f}%")
            print()

    # Plot results
    fig, axes = plt.subplots(4, 1, figsize=(12, 10), sharex=True)

    # Plot 1: Position and velocity
    ax1 = axes[0]
    ax1.plot(time_arr, position_arr, "b-", label="Position", linewidth=1.5)
    ax1.set_ylabel("Position (rad)", color="b")
    ax1.tick_params(axis="y", labelcolor="b")
    ax1.grid(True, alpha=0.3)
    ax1.legend(loc="upper left")

    ax1b = ax1.twinx()
    ax1b.plot(time_arr, velocity_arr, "r-", label="Velocity", linewidth=1.5, alpha=0.7)
    ax1b.set_ylabel("Velocity (rad/s)", color="r")
    ax1b.tick_params(axis="y", labelcolor="r")
    ax1b.legend(loc="upper right")

    ax1.set_title(
        "Trajectory: Motion Test with Load Application", fontsize=12, fontweight="bold"
    )

    # Plot 2: Load estimates
    ax2 = axes[1]
    ax2.plot(
        time_arr, external_load_arr, "k--", label="True External Load", linewidth=2
    )
    ax2.plot(
        time_arr, observer_estimate_arr, "g-", label="Observer Estimate", linewidth=1.5
    )
    ax2.plot(
        time_arr,
        baseline_estimate_arr,
        "orange",
        label="Baseline Estimate",
        linewidth=1.5,
        alpha=0.7,
    )
    ax2.set_ylabel("Load (Nm)")
    ax2.legend(loc="upper right")
    ax2.grid(True, alpha=0.3)
    ax2.set_title("Load Estimation Comparison", fontsize=12, fontweight="bold")

    # Plot 3: Estimation errors
    ax3 = axes[2]
    ax3.plot(
        time_arr, observer_error * 1000, "g-", label="Observer Error", linewidth=1.5
    )
    ax3.plot(
        time_arr,
        baseline_error * 1000,
        "orange",
        label="Baseline Error",
        linewidth=1.5,
        alpha=0.7,
    )
    ax3.axhline(0, color="k", linestyle="--", linewidth=0.5)
    ax3.set_ylabel("Error (mNm)")
    ax3.legend(loc="upper right")
    ax3.grid(True, alpha=0.3)
    ax3.set_title("Estimation Errors", fontsize=12, fontweight="bold")

    # Plot 4: Observer diagnostics
    ax4 = axes[3]
    diagnostics = observer.get_diagnostics()
    if len(diagnostics["tau_motor"]) > 0:
        # Downsample for plotting
        downsample = 10
        t_diag = time_arr
        ax4.plot(
            t_diag,
            diagnostics["tau_motor"][::downsample],
            label="Motor Torque",
            linewidth=1,
            alpha=0.7,
        )
        ax4.plot(
            t_diag,
            diagnostics["tau_motion"][::downsample],
            label="Motion Torque (J·α+b·ω)",
            linewidth=1,
            alpha=0.7,
        )
        ax4.plot(
            t_diag,
            diagnostics["tau_friction"][::downsample],
            label="Friction Torque",
            linewidth=1,
            alpha=0.7,
        )
        ax4.set_ylabel("Torque (Nm)")
        ax4.legend(loc="upper right", fontsize=8)
        ax4.grid(True, alpha=0.3)
        ax4.set_title(
            "Observer Diagnostics (Torque Components)", fontsize=12, fontweight="bold"
        )

    ax4.set_xlabel("Time (s)")

    plt.tight_layout()

    # Save plot
    output_file = "demo_results/disturbance_observer_comparison.png"
    Path("demo_results").mkdir(exist_ok=True)
    plt.savefig(output_file, dpi=150, bbox_inches="tight")
    print(f"📊 Plot saved to: {output_file}")
    print()

    print("=" * 80)
    print("✅ Test Complete!")
    print("=" * 80)
    print()
    print("🎯 Key Advantages of Disturbance Observer:")
    print("  1. Works during motion (not just steady state)")
    print("  2. Better noise rejection with physics-based model")
    print("  3. Separates friction from external load")
    print("  4. More robust to parameter variations")
    print()
    print(
        f"Overall Improvement: {(baseline_rmse - observer_rmse) / baseline_rmse * 100:.1f}%"
    )


if __name__ == "__main__":
    test_observer_vs_baseline()
