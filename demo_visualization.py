#!/usr/bin/env python3
"""
Demo FOC Visualization System

Generates example FOC telemetry data and visualization reports
to demonstrate the test visualization capabilities.
"""

import sys
import numpy as np
from pathlib import Path

# Add renode/tests to path
sys.path.insert(0, str(Path(__file__).parent / "renode" / "tests"))

from test_data_collector import TestDataCollector
from test_report_generator import FocTestReportGenerator, generate_test_suite_summary


def simulate_trapezoidal_motion():
    """Simulate trapezoidal motion profile with FOC control."""
    print("\n📊 Simulating Trapezoidal Motion Profile...")

    collector = TestDataCollector("demo_trapezoidal_profile")

    # Motion parameters
    target = 1.57  # 90 degrees
    max_vel = 2.0  # rad/s
    max_accel = 5.0  # rad/s²

    # Calculate motion phases
    t_accel = max_vel / max_accel
    t_coast = 0.0  # No coast for this profile
    t_decel = t_accel

    x_accel = 0.5 * max_accel * t_accel**2
    if 2 * x_accel < target:
        # Has coast phase
        t_coast = (target - 2 * x_accel) / max_vel
    else:
        # Pure triangular
        t_accel = np.sqrt(target / max_accel)
        t_decel = t_accel
        max_vel = max_accel * t_accel

    # Simulate at 10 kHz
    dt = 0.0001
    duration = t_accel + t_coast + t_decel + 0.2  # Add settling time
    n_samples = int(duration / dt)

    position = 0.0
    velocity = 0.0

    for i in range(n_samples):
        t = i * dt

        # Motion profile
        if t < t_accel:
            # Acceleration phase
            target_vel = max_accel * t
            target_pos = 0.5 * max_accel * t**2
        elif t < t_accel + t_coast:
            # Coast phase
            target_vel = max_vel
            target_pos = 0.5 * max_accel * t_accel**2 + max_vel * (t - t_accel)
        elif t < t_accel + t_coast + t_decel:
            # Deceleration phase
            t_dec = t - t_accel - t_coast
            target_vel = max_vel - max_accel * t_dec
            target_pos = (
                0.5 * max_accel * t_accel**2
                + max_vel * t_coast
                + max_vel * t_dec
                - 0.5 * max_accel * t_dec**2
            )
        else:
            # Settling
            target_vel = 0.0
            target_pos = target

        # Simulate FOC PI controller (with realistic lag)
        pos_error = target_pos - position
        vel_error = target_vel - velocity

        # PI gains
        kp_pos = 20.0
        kp_vel = 0.5
        ki_vel = 2.0

        # Control law
        velocity += (kp_pos * pos_error + kp_vel * vel_error) * dt
        position += velocity * dt

        # Current (I_q) proportional to acceleration + friction
        accel = kp_pos * pos_error + kp_vel * vel_error
        i_q = 0.1 * accel + 0.05 * velocity  # Motor model: τ = kt * i_q
        i_d = 0.0  # Field weakening not used

        # Load estimation (from current)
        load = 0.15 * i_q

        # PWM duty cycles (3-phase, simplified)
        theta = position  # Electrical angle
        duty_a = 0.5 + 0.3 * i_q * np.cos(theta)
        duty_b = 0.5 + 0.3 * i_q * np.cos(theta - 2 * np.pi / 3)
        duty_c = 0.5 + 0.3 * i_q * np.cos(theta + 2 * np.pi / 3)

        # Clamp
        duty_a = np.clip(duty_a, 0.0, 1.0)
        duty_b = np.clip(duty_b, 0.0, 1.0)
        duty_c = np.clip(duty_c, 0.0, 1.0)

        # Temperature (slowly rising with I²R losses)
        temp = 25.0 + 5.0 * np.tanh(t * 0.5)

        # Health score (slowly degrading with stress)
        health = 100.0 - 2.0 * np.tanh(t * 0.2)

        # Record every 10th sample (1 kHz effective rate for demo)
        if i % 10 == 0:
            collector.add_from_peripherals(
                encoder_position=position,
                encoder_velocity=velocity,
                adc_i_q=i_q,
                adc_i_d=i_d,
                motor_pwm_a=duty_a,
                motor_pwm_b=duty_b,
                motor_pwm_c=duty_c,
                target_position=target_pos,
                target_velocity=target_vel,
                load_estimate=load,
                temperature=temp,
                health_score=health,
            )

    # Save
    output_dir = Path("demo_results")
    output_dir.mkdir(exist_ok=True)

    collector.save_json(str(output_dir / "demo_trapezoidal_profile.json"))
    collector.save_pandas_csv(str(output_dir / "demo_trapezoidal_profile.csv"))

    print(f"   ✓ Generated {len(collector.snapshots)} samples")
    print(f"   ✓ Duration: {duration:.2f} s")

    return str(output_dir / "demo_trapezoidal_profile.json")


def simulate_adaptive_control_load_step():
    """Simulate adaptive control response to load disturbance."""
    print("\n📊 Simulating Adaptive Control Load Step...")

    collector = TestDataCollector("demo_adaptive_load_step")

    dt = 0.0001
    duration = 0.6  # 600 ms
    n_samples = int(duration / dt)

    position = 0.0
    velocity = 0.0
    target_pos = 1.0  # Hold position at 1.0 rad

    # coolStep state
    load_estimate = 0.0
    coolstep_enabled = True
    current_reduction_factor = 1.0

    for i in range(n_samples):
        t = i * dt

        # Apply external load disturbance at t=0.2s
        if 0.2 <= t < 0.4:
            external_load = 0.3  # 0.3 Nm disturbance
        else:
            external_load = 0.0

        # Position controller
        pos_error = target_pos - position
        kp = 20.0
        kd = 2.0

        accel = kp * pos_error - kd * velocity
        velocity += accel * dt
        position += velocity * dt

        # Current (with external load)
        i_q_base = 0.1 * accel + 0.05 * velocity + external_load / 0.15

        # Load estimation (low-pass filter on current)
        alpha = 0.01
        load_estimate = alpha * (0.15 * i_q_base) + (1 - alpha) * load_estimate

        # coolStep: Reduce current when load is steady
        if coolstep_enabled:
            # If load is stable and high, reduce current
            if load_estimate > 0.1:
                # Reduce by up to 30%
                reduction = min(0.3, 0.1 * (load_estimate - 0.1))
                current_reduction_factor = 1.0 - reduction
            else:
                current_reduction_factor = 1.0
        else:
            current_reduction_factor = 1.0

        i_q = i_q_base * current_reduction_factor
        i_d = 0.0

        # PWM
        theta = position
        duty_a = 0.5 + 0.3 * i_q * np.cos(theta)
        duty_b = 0.5 + 0.3 * i_q * np.cos(theta - 2 * np.pi / 3)
        duty_c = 0.5 + 0.3 * i_q * np.cos(theta + 2 * np.pi / 3)

        duty_a = np.clip(duty_a, 0.0, 1.0)
        duty_b = np.clip(duty_b, 0.0, 1.0)
        duty_c = np.clip(duty_c, 0.0, 1.0)

        # Temperature rises with current
        temp = 25.0 + 10.0 * (i_q / 2.0) ** 2

        # Health degrades with high load
        health = 100.0 - 10.0 * (load_estimate / 0.5) ** 2
        health = max(health, 60.0)

        # Record every 10th sample
        if i % 10 == 0:
            collector.add_from_peripherals(
                encoder_position=position,
                encoder_velocity=velocity,
                adc_i_q=i_q,
                adc_i_d=i_d,
                motor_pwm_a=duty_a,
                motor_pwm_b=duty_b,
                motor_pwm_c=duty_c,
                target_position=target_pos,
                target_velocity=0.0,
                load_estimate=load_estimate,
                temperature=temp,
                health_score=health,
            )

    # Save
    output_dir = Path("demo_results")
    output_dir.mkdir(exist_ok=True)

    collector.save_json(str(output_dir / "demo_adaptive_load_step.json"))
    collector.save_pandas_csv(str(output_dir / "demo_adaptive_load_step.csv"))

    print(f"   ✓ Generated {len(collector.snapshots)} samples")
    print(f"   ✓ Load step: 0→0.3→0 Nm")

    return str(output_dir / "demo_adaptive_load_step.json")


def simulate_high_speed_motion():
    """Simulate high-speed motion with saturation effects."""
    print("\n📊 Simulating High-Speed Motion...")

    collector = TestDataCollector("demo_high_speed_motion")

    dt = 0.0001
    duration = 1.0
    n_samples = int(duration / dt)

    position = 0.0
    velocity = 0.0
    target = 6.28  # 360 degrees
    max_vel = 10.0  # Very fast
    max_accel = 50.0

    for i in range(n_samples):
        t = i * dt

        # S-curve profile (simplified)
        t_jerk = 0.05
        t_accel = max_vel / max_accel

        if t < t_jerk:
            # Jerk phase
            jerk = max_accel / t_jerk
            target_accel = jerk * t
            target_vel = 0.5 * jerk * t**2
            target_pos = (1 / 6) * jerk * t**3
        elif t < t_accel:
            # Constant accel
            target_accel = max_accel
            target_vel = max_accel * t
            target_pos = 0.5 * max_accel * t**2
        else:
            # Coast/decel (simplified)
            target_accel = 0.0
            target_vel = max_vel
            target_pos = 0.5 * max_accel * t_accel**2 + max_vel * (t - t_accel)

        target_pos = min(target_pos, target)

        # Controller (with saturation)
        pos_error = target_pos - position
        vel_error = target_vel - velocity

        accel = 30.0 * pos_error + 1.0 * vel_error
        accel = np.clip(accel, -max_accel, max_accel)  # Saturation

        velocity += accel * dt
        velocity = np.clip(velocity, -max_vel, max_vel)
        position += velocity * dt

        # High currents
        i_q = 0.2 * accel + 0.1 * velocity
        i_q = np.clip(i_q, -5.0, 5.0)  # Current limit
        i_d = 0.0

        # Saturation indicator
        saturation = abs(i_q) > 4.5

        # Load
        load = 0.15 * i_q

        # PWM (with saturation)
        theta = position
        duty_a = 0.5 + 0.4 * i_q * np.cos(theta)
        duty_b = 0.5 + 0.4 * i_q * np.cos(theta - 2 * np.pi / 3)
        duty_c = 0.5 + 0.4 * i_q * np.cos(theta + 2 * np.pi / 3)

        # Hard saturation
        duty_a = np.clip(duty_a, 0.0, 1.0)
        duty_b = np.clip(duty_b, 0.0, 1.0)
        duty_c = np.clip(duty_c, 0.0, 1.0)

        # Temperature rises quickly
        temp = 25.0 + 20.0 * np.tanh(t * 2.0)

        # Health degrades with high speed
        health = 100.0 - 15.0 * np.tanh(velocity / 5.0)

        # Record
        if i % 10 == 0:
            collector.add_from_peripherals(
                encoder_position=position,
                encoder_velocity=velocity,
                adc_i_q=i_q,
                adc_i_d=i_d,
                motor_pwm_a=duty_a,
                motor_pwm_b=duty_b,
                motor_pwm_c=duty_c,
                target_position=target_pos,
                target_velocity=target_vel,
                load_estimate=load,
                temperature=temp,
                health_score=health,
            )

    # Save
    output_dir = Path("demo_results")
    collector.save_json(str(output_dir / "demo_high_speed_motion.json"))
    collector.save_pandas_csv(str(output_dir / "demo_high_speed_motion.csv"))

    print(f"   ✓ Generated {len(collector.snapshots)} samples")
    print(f"   ✓ Max velocity: {max_vel} rad/s")

    return str(output_dir / "demo_high_speed_motion.json")


def main():
    """Generate demo data and reports."""
    print("=" * 60)
    print("FOC Test Visualization System - Demo")
    print("=" * 60)

    # Generate demo data
    print("\n🔧 Generating demo FOC telemetry data...")

    json_files = []
    json_files.append(simulate_trapezoidal_motion())
    json_files.append(simulate_adaptive_control_load_step())
    json_files.append(simulate_high_speed_motion())

    # Generate reports
    print("\n📊 Generating visualization reports...")

    for json_file in json_files:
        test_name = Path(json_file).stem
        pdf_file = str(Path(json_file).with_suffix("")) + "_report.pdf"

        print(f"\n   Generating: {test_name}_report.pdf")

        try:
            generator = FocTestReportGenerator(json_file)
            generator.generate_pdf(pdf_file)
            print(f"   ✓ Report saved")
        except Exception as e:
            print(f"   ❌ Failed: {e}")

    # Generate suite summary
    print("\n📊 Generating test suite summary...")
    try:
        generate_test_suite_summary(
            "demo_results", "demo_results/demo_suite_summary.pdf"
        )
        print("   ✓ Suite summary generated")
    except Exception as e:
        print(f"   ❌ Failed: {e}")

    # Summary
    print("\n" + "=" * 60)
    print("Demo Complete!")
    print("=" * 60)
    print("\nGenerated files in demo_results/:")
    print("  📊 JSON data files")
    print("  📈 CSV files (pandas format)")
    print("  📄 PDF reports with FOC plots")
    print("  📑 Test suite summary")
    print("\nOpen the PDFs to see:")
    print("  - Motion tracking (position, velocity)")
    print("  - Tracking error analysis")
    print("  - FOC d-q currents")
    print("  - 3-phase PWM duty cycles")
    print("  - Load estimation & temperature")
    print("  - Health score trends")
    print("  - Phase diagrams")
    print("\n" + "=" * 60)


if __name__ == "__main__":
    main()
