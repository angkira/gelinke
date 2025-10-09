#!/usr/bin/env python3
"""
FOC Monitoring Validation Script
Performs detailed validation of FOC test results against expected behavior
"""

import json
import numpy as np
from pathlib import Path


def load_test_data(json_file):
    """Load test data from JSON file."""
    with open(json_file, "r") as f:
        data = json.load(f)
    return data


def calculate_statistics(data):
    """Calculate statistics from test data."""
    samples = data["samples"]

    # Extract arrays
    position = np.array([s["position"] for s in samples])
    target_position = np.array([s["target_position"] for s in samples])
    velocity = np.array([s["velocity"] for s in samples])
    target_velocity = np.array([s["target_velocity"] for s in samples])
    i_q = np.array([s["i_q"] for s in samples])
    i_d = np.array([s["i_d"] for s in samples])
    pwm_a = np.array([s["pwm_duty_a"] for s in samples])
    pwm_b = np.array([s["pwm_duty_b"] for s in samples])
    pwm_c = np.array([s["pwm_duty_c"] for s in samples])
    load_estimate = np.array([s["load_estimate"] for s in samples])
    temperature = np.array([s["temperature"] for s in samples])
    health_score = np.array([s["health_score"] for s in samples])

    # Calculate tracking errors
    pos_error = target_position - position
    vel_error = target_velocity - velocity

    # Convert position error to degrees for display
    pos_error_deg = np.rad2deg(pos_error)

    # Calculate RMS and max errors
    rms_error_rad = np.sqrt(np.mean(pos_error**2))
    rms_error_deg = np.rad2deg(rms_error_rad)
    max_error_deg = np.max(np.abs(pos_error_deg))
    mean_error_deg = np.mean(np.abs(pos_error_deg))

    # Current magnitude
    i_magnitude = np.sqrt(i_q**2 + i_d**2)

    stats = {
        "sample_count": len(samples),
        "duration": data.get("test_duration", 0),
        "position": {
            "min": np.min(position),
            "max": np.max(position),
            "std": np.std(position),
            "final": position[-1] if len(position) > 0 else 0,
        },
        "velocity": {
            "min": np.min(velocity),
            "max": np.max(velocity),
            "mean": np.mean(np.abs(velocity)),
        },
        "tracking_error": {
            "rms_rad": rms_error_rad,
            "rms_deg": rms_error_deg,
            "max_deg": max_error_deg,
            "mean_deg": mean_error_deg,
        },
        "current": {
            "i_q_peak": np.max(np.abs(i_q)),
            "i_q_mean": np.mean(np.abs(i_q)),
            "i_d_peak": np.max(np.abs(i_d)),
            "i_d_mean": np.mean(np.abs(i_d)),
            "magnitude_peak": np.max(i_magnitude),
            "magnitude_rms": np.sqrt(np.mean(i_magnitude**2)),
        },
        "pwm": {
            "a_min": np.min(pwm_a),
            "a_max": np.max(pwm_a),
            "b_min": np.min(pwm_b),
            "b_max": np.max(pwm_b),
            "c_min": np.min(pwm_c),
            "c_max": np.max(pwm_c),
        },
        "load": {
            "min": np.min(load_estimate),
            "max": np.max(load_estimate),
            "mean": np.mean(load_estimate),
        },
        "temperature": {
            "min": np.min(temperature),
            "max": np.max(temperature),
            "final": temperature[-1] if len(temperature) > 0 else 0,
        },
        "health": {
            "initial": health_score[0] if len(health_score) > 0 else 0,
            "final": health_score[-1] if len(health_score) > 0 else 0,
            "min": np.min(health_score),
        },
    }

    return stats


def validate_test_1_trapezoidal(stats, samples):
    """Validate Test 1: Trapezoidal Motion Profile."""

    issues = []
    warnings = []
    passes = []

    # Expected parameters from code
    expected_target = 1.57  # rad (90 degrees)
    expected_max_vel = 2.0  # rad/s
    expected_max_accel = 5.0  # rad/s²

    # Check sample count (should be ~1385)
    if 1300 <= stats["sample_count"] <= 1400:
        passes.append(f"Sample count: {stats['sample_count']} ✅ (expected ~1385)")
    else:
        issues.append(f"Sample count: {stats['sample_count']} ❌ (expected ~1385)")

    # Check position reaches target
    final_pos = stats["position"]["final"]
    pos_error = abs(final_pos - expected_target)
    if pos_error < 0.1:  # 0.1 rad tolerance
        passes.append(
            f"Position reached target: {final_pos:.4f} rad vs {expected_target} rad ✅"
        )
    else:
        issues.append(
            f"Position didn't reach target: {final_pos:.4f} rad vs {expected_target} rad ❌"
        )

    # Check position range
    if abs(stats["position"]["min"]) < 0.1 and 1.5 <= stats["position"]["max"] <= 1.6:
        passes.append(
            f"Position range: [{stats['position']['min']:.2f}, {stats['position']['max']:.2f}] rad ✅"
        )
    else:
        warnings.append(
            f"Position range: [{stats['position']['min']:.2f}, {stats['position']['max']:.2f}] rad ⚠️"
        )

    # Check max velocity
    if 1.8 <= stats["velocity"]["max"] <= 2.2:
        passes.append(
            f"Max velocity: {stats['velocity']['max']:.2f} rad/s ✅ (expected ~{expected_max_vel})"
        )
    else:
        issues.append(
            f"Max velocity: {stats['velocity']['max']:.2f} rad/s ❌ (expected ~{expected_max_vel})"
        )

    # Check tracking error
    if stats["tracking_error"]["rms_deg"] < 0.5:
        passes.append(
            f"RMS error: {stats['tracking_error']['rms_deg']:.3f}° ✅ (< 0.5°)"
        )
    else:
        warnings.append(
            f"RMS error: {stats['tracking_error']['rms_deg']:.3f}° ⚠️ (> 0.5°)"
        )

    if stats["tracking_error"]["max_deg"] < 5.0:
        passes.append(f"Max error: {stats['tracking_error']['max_deg']:.2f}° ✅ (< 5°)")
    else:
        warnings.append(
            f"Max error: {stats['tracking_error']['max_deg']:.2f}° ⚠️ (> 5°)"
        )

    # Check I_q current
    if 0.4 <= stats["current"]["i_q_peak"] <= 1.2:
        passes.append(
            f"Peak I_q: {stats['current']['i_q_peak']:.3f} A ✅ (expected 0.5-1.0 A)"
        )
    else:
        warnings.append(
            f"Peak I_q: {stats['current']['i_q_peak']:.3f} A ⚠️ (expected 0.5-1.0 A)"
        )

    if 0.15 <= stats["current"]["i_q_mean"] <= 0.6:
        passes.append(
            f"Mean I_q: {stats['current']['i_q_mean']:.3f} A ✅ (expected 0.2-0.5 A)"
        )
    else:
        warnings.append(
            f"Mean I_q: {stats['current']['i_q_mean']:.3f} A ⚠️ (expected 0.2-0.5 A)"
        )

    # Check I_d (should be ~0)
    if stats["current"]["i_d_peak"] < 0.1:
        passes.append(
            f"Peak I_d: {stats['current']['i_d_peak']:.4f} A ✅ (< 0.1 A, field weakening not used)"
        )
    else:
        issues.append(
            f"Peak I_d: {stats['current']['i_d_peak']:.4f} A ❌ (should be ~0)"
        )

    # Check PWM saturation
    if 0.0 <= stats["pwm"]["a_min"] and stats["pwm"]["a_max"] <= 1.0:
        if 0.0 <= stats["pwm"]["b_min"] and stats["pwm"]["b_max"] <= 1.0:
            if 0.0 <= stats["pwm"]["c_min"] and stats["pwm"]["c_max"] <= 1.0:
                passes.append(f"PWM in range [0, 1] ✅")
            else:
                issues.append(
                    f"PWM C out of range: [{stats['pwm']['c_min']:.3f}, {stats['pwm']['c_max']:.3f}] ❌"
                )
        else:
            issues.append(
                f"PWM B out of range: [{stats['pwm']['b_min']:.3f}, {stats['pwm']['b_max']:.3f}] ❌"
            )
    else:
        issues.append(
            f"PWM A out of range: [{stats['pwm']['a_min']:.3f}, {stats['pwm']['a_max']:.3f}] ❌"
        )

    # Check temperature rise
    temp_rise = stats["temperature"]["final"] - stats["temperature"]["min"]
    if 3 <= temp_rise <= 10:
        passes.append(f"Temperature rise: {temp_rise:.1f}°C ✅ (expected ~5°C)")
    else:
        warnings.append(f"Temperature rise: {temp_rise:.1f}°C ⚠️ (expected ~5°C)")

    if stats["temperature"]["min"] >= 20 and stats["temperature"]["max"] <= 50:
        passes.append(
            f"Temperature range: [{stats['temperature']['min']:.1f}, {stats['temperature']['max']:.1f}]°C ✅"
        )
    else:
        warnings.append(
            f"Temperature range: [{stats['temperature']['min']:.1f}, {stats['temperature']['max']:.1f}]°C ⚠️"
        )

    # Check health score degradation
    health_degradation = stats["health"]["initial"] - stats["health"]["final"]
    if 1 <= health_degradation <= 5:
        passes.append(f"Health degradation: {health_degradation:.1f} ✅ (expected ~2)")
    else:
        warnings.append(f"Health degradation: {health_degradation:.1f} ⚠️ (expected ~2)")

    return {
        "passes": passes,
        "warnings": warnings,
        "issues": issues,
    }


def validate_test_2_adaptive(stats, samples):
    """Validate Test 2: Adaptive Control Load Step."""

    issues = []
    warnings = []
    passes = []

    # Expected parameters
    expected_target = 1.0  # rad
    expected_duration = 0.6  # s
    expected_load_step = 0.3  # Nm at t=0.2s

    # Check sample count
    if 550 <= stats["sample_count"] <= 650:
        passes.append(f"Sample count: {stats['sample_count']} ✅ (expected ~600)")
    else:
        warnings.append(f"Sample count: {stats['sample_count']} ⚠️ (expected ~600)")

    # Check position hold
    pos_std = stats["position"]["std"]
    if pos_std < 0.1:
        passes.append(f"Position std dev: {pos_std:.4f} rad ✅ (< 0.1 rad, good hold)")
    else:
        warnings.append(f"Position std dev: {pos_std:.4f} rad ⚠️ (> 0.1 rad)")

    # Check load estimation
    if 0.25 <= stats["load"]["max"] <= 0.35:
        passes.append(
            f"Peak load estimate: {stats['load']['max']:.3f} Nm ✅ (expected ~0.3 Nm)"
        )
    else:
        warnings.append(
            f"Peak load estimate: {stats['load']['max']:.3f} Nm ⚠️ (expected ~0.3 Nm)"
        )

    # Check current response to load
    if 0.8 <= stats["current"]["i_q_peak"] <= 2.5:
        passes.append(
            f"Peak I_q: {stats['current']['i_q_peak']:.3f} A ✅ (expected 1.0-2.0 A)"
        )
    else:
        warnings.append(
            f"Peak I_q: {stats['current']['i_q_peak']:.3f} A ⚠️ (expected 1.0-2.0 A)"
        )

    # Analyze load step response
    # Split data into phases
    n_samples = len(samples)
    phase1_end = int(0.2 / 0.6 * n_samples)  # t < 0.2s
    phase2_start = phase1_end
    phase2_end = int(0.4 / 0.6 * n_samples)  # 0.2s <= t < 0.4s
    phase3_start = phase2_end  # t >= 0.4s

    if n_samples > phase2_end:
        i_q_phase1 = [s["i_q"] for s in samples[:phase1_end]]
        i_q_phase2 = [s["i_q"] for s in samples[phase2_start:phase2_end]]
        i_q_phase3 = [s["i_q"] for s in samples[phase3_start:]]

        load_phase1 = [s["load_estimate"] for s in samples[:phase1_end]]
        load_phase2 = [s["load_estimate"] for s in samples[phase2_start:phase2_end]]
        load_phase3 = [s["load_estimate"] for s in samples[phase3_start:]]

        i_q_phase1_mean = np.mean(np.abs(i_q_phase1)) if i_q_phase1 else 0
        i_q_phase2_mean = np.mean(np.abs(i_q_phase2)) if i_q_phase2 else 0
        i_q_phase3_mean = np.mean(np.abs(i_q_phase3)) if i_q_phase3 else 0

        load_phase1_mean = np.mean(load_phase1) if load_phase1 else 0
        load_phase2_mean = np.mean(load_phase2) if load_phase2 else 0
        load_phase3_mean = np.mean(load_phase3) if load_phase3 else 0

        # Check current increase during load
        if i_q_phase2_mean > i_q_phase1_mean * 1.5:
            passes.append(
                f"Current increased during load step ✅ (Phase1: {i_q_phase1_mean:.3f}A → Phase2: {i_q_phase2_mean:.3f}A)"
            )
        else:
            warnings.append(
                f"Current didn't increase enough during load ⚠️ (Phase1: {i_q_phase1_mean:.3f}A → Phase2: {i_q_phase2_mean:.3f}A)"
            )

        # Check load estimation tracking
        if load_phase2_mean > 0.15 and load_phase2_mean > load_phase1_mean * 3:
            passes.append(
                f"Load estimation tracks external load ✅ (Phase1: {load_phase1_mean:.3f}Nm → Phase2: {load_phase2_mean:.3f}Nm)"
            )
        else:
            issues.append(
                f"Load estimation doesn't track external load ❌ (Phase1: {load_phase1_mean:.3f}Nm → Phase2: {load_phase2_mean:.3f}Nm)"
            )

        # Check current returns to baseline after load removal
        if i_q_phase3_mean < i_q_phase2_mean * 0.7:
            passes.append(
                f"Current returned to baseline after load removal ✅ (Phase3: {i_q_phase3_mean:.3f}A)"
            )
        else:
            warnings.append(
                f"Current didn't return to baseline ⚠️ (Phase3: {i_q_phase3_mean:.3f}A)"
            )

    # Check temperature spike
    temp_rise = stats["temperature"]["max"] - stats["temperature"]["min"]
    if temp_rise > 5:
        passes.append(f"Temperature spiked during load: {temp_rise:.1f}°C ✅")
    else:
        warnings.append(f"Temperature didn't spike much: {temp_rise:.1f}°C ⚠️")

    # Check health score degradation and recovery
    health_drop = stats["health"]["initial"] - stats["health"]["min"]
    if health_drop >= 10:
        passes.append(f"Health score degraded under load: {health_drop:.1f} points ✅")
    else:
        warnings.append(f"Health score didn't degrade much: {health_drop:.1f} points ⚠️")

    # Check PWM
    if 0.0 <= stats["pwm"]["a_min"] and stats["pwm"]["a_max"] <= 1.0:
        passes.append(f"PWM in range [0, 1] ✅")
    else:
        issues.append(f"PWM out of range ❌")

    return {
        "passes": passes,
        "warnings": warnings,
        "issues": issues,
    }


def validate_test_3_high_speed(stats, samples):
    """Validate Test 3: High-Speed Motion."""

    issues = []
    warnings = []
    passes = []

    # Expected parameters
    expected_target = 6.28  # rad (360 degrees)
    expected_max_vel = 10.0  # rad/s
    expected_max_accel = 50.0  # rad/s²
    expected_max_current = 5.0  # A (saturation limit)

    # Check sample count
    if 900 <= stats["sample_count"] <= 1100:
        passes.append(f"Sample count: {stats['sample_count']} ✅ (expected ~1000)")
    else:
        warnings.append(f"Sample count: {stats['sample_count']} ⚠️ (expected ~1000)")

    # Check position reaches target
    final_pos = stats["position"]["final"]
    pos_error = abs(final_pos - expected_target)
    if pos_error < 0.3:
        passes.append(
            f"Position reached target: {final_pos:.2f} rad vs {expected_target} rad ✅"
        )
    else:
        warnings.append(
            f"Position close to target: {final_pos:.2f} rad vs {expected_target} rad ⚠️"
        )

    # Check max velocity
    if 9.0 <= stats["velocity"]["max"] <= 10.5:
        passes.append(
            f"Max velocity: {stats['velocity']['max']:.2f} rad/s ✅ (expected ~{expected_max_vel})"
        )
    elif stats["velocity"]["max"] > 10.5:
        issues.append(
            f"Max velocity exceeded limit: {stats['velocity']['max']:.2f} rad/s ❌ (max {expected_max_vel})"
        )
    else:
        warnings.append(
            f"Max velocity: {stats['velocity']['max']:.2f} rad/s ⚠️ (expected ~{expected_max_vel})"
        )

    # Check current saturation
    if 4.5 <= stats["current"]["i_q_peak"] <= 5.0:
        passes.append(
            f"Peak I_q saturated at: {stats['current']['i_q_peak']:.2f} A ✅ (expected ~5.0 A)"
        )
    elif stats["current"]["i_q_peak"] > 5.0:
        issues.append(
            f"Peak I_q exceeded saturation: {stats['current']['i_q_peak']:.2f} A ❌ (max 5.0 A)"
        )
    else:
        warnings.append(
            f"Peak I_q: {stats['current']['i_q_peak']:.2f} A ⚠️ (expected ~5.0 A saturation)"
        )

    # Check PWM saturation
    pwm_saturated = False
    if stats["pwm"]["a_min"] <= 0.05 or stats["pwm"]["a_max"] >= 0.95:
        pwm_saturated = True
    if stats["pwm"]["b_min"] <= 0.05 or stats["pwm"]["b_max"] >= 0.95:
        pwm_saturated = True
    if stats["pwm"]["c_min"] <= 0.05 or stats["pwm"]["c_max"] >= 0.95:
        pwm_saturated = True

    if pwm_saturated:
        passes.append(f"PWM shows saturation effects ✅")
    else:
        warnings.append(f"PWM doesn't show saturation (expected at high speed) ⚠️")

    # Check PWM bounds
    if (
        0.0 <= stats["pwm"]["a_min"]
        and stats["pwm"]["a_max"] <= 1.0
        and 0.0 <= stats["pwm"]["b_min"]
        and stats["pwm"]["b_max"] <= 1.0
        and 0.0 <= stats["pwm"]["c_min"]
        and stats["pwm"]["c_max"] <= 1.0
    ):
        passes.append(f"PWM in valid range [0, 1] ✅")
    else:
        issues.append(f"PWM out of valid range ❌")

    # Check temperature rapid rise
    temp_rise = stats["temperature"]["final"] - stats["temperature"]["min"]
    if temp_rise >= 15:
        passes.append(
            f"Temperature rose rapidly: {temp_rise:.1f}°C ✅ (expected 15-20°C)"
        )
    else:
        warnings.append(f"Temperature rise: {temp_rise:.1f}°C ⚠️ (expected 15-20°C)")

    if stats["temperature"]["max"] <= 50:
        passes.append(
            f"Temperature stayed below 50°C: {stats['temperature']['max']:.1f}°C ✅"
        )
    else:
        warnings.append(f"Temperature high: {stats['temperature']['max']:.1f}°C ⚠️")

    # Check health degradation
    health_drop = stats["health"]["initial"] - stats["health"]["final"]
    if 10 <= health_drop <= 20:
        passes.append(
            f"Health degraded significantly: {health_drop:.1f} points ✅ (expected ~15)"
        )
    else:
        warnings.append(
            f"Health degradation: {health_drop:.1f} points ⚠️ (expected ~15)"
        )

    # Check tracking error (should be higher due to saturation)
    if stats["tracking_error"]["rms_deg"] < 2.0:
        passes.append(f"RMS error: {stats['tracking_error']['rms_deg']:.3f}° ✅ (< 2°)")
    else:
        warnings.append(
            f"RMS error: {stats['tracking_error']['rms_deg']:.3f}° ⚠️ (> 2°, acceptable with saturation)"
        )

    return {
        "passes": passes,
        "warnings": warnings,
        "issues": issues,
    }


def print_test_results(test_name, validation_results, stats):
    """Print formatted test results."""
    print(f"\n{'='*80}")
    print(f"## {test_name}")
    print(f"{'='*80}")

    print(f"\n### 📊 Summary Statistics:")
    print(f"  Samples: {stats['sample_count']}")
    print(f"  Duration: {stats['duration']:.2f} s")
    print(
        f"  Position: [{stats['position']['min']:.3f}, {stats['position']['max']:.3f}] rad (std: {stats['position']['std']:.3f})"
    )
    print(
        f"  Velocity: max {stats['velocity']['max']:.2f} rad/s, mean {stats['velocity']['mean']:.2f} rad/s"
    )
    print(
        f"  Tracking Error: RMS {stats['tracking_error']['rms_deg']:.3f}°, max {stats['tracking_error']['max_deg']:.2f}°"
    )
    print(
        f"  Current I_q: peak {stats['current']['i_q_peak']:.3f} A, mean {stats['current']['i_q_mean']:.3f} A"
    )
    print(
        f"  Current I_d: peak {stats['current']['i_d_peak']:.4f} A, mean {stats['current']['i_d_mean']:.4f} A"
    )
    print(
        f"  Temperature: [{stats['temperature']['min']:.1f}, {stats['temperature']['max']:.1f}]°C"
    )
    print(
        f"  Health: {stats['health']['initial']:.1f} → {stats['health']['final']:.1f} (drop: {stats['health']['initial'] - stats['health']['final']:.1f})"
    )

    print(f"\n### ✅ What works correctly:")
    for item in validation_results["passes"]:
        print(f"  - {item}")

    if validation_results["warnings"]:
        print(f"\n### ⚠️  Warnings:")
        for item in validation_results["warnings"]:
            print(f"  - {item}")

    if validation_results["issues"]:
        print(f"\n### ❌ Critical Issues:")
        for item in validation_results["issues"]:
            print(f"  - {item}")

    # Verdict
    print(f"\n### 🎯 Verdict: ", end="")
    if not validation_results["issues"]:
        if len(validation_results["warnings"]) <= 2:
            print("✅ PASS")
        else:
            print("⚠️  PASS WITH WARNINGS")
    else:
        print("❌ FAIL")


def main():
    """Main validation function."""
    print("=" * 80)
    print("# 🔍 FOC Monitoring Validation Report")
    print("=" * 80)

    results_dir = Path("demo_results")

    # Test 1: Trapezoidal Motion
    test1_data = load_test_data(results_dir / "demo_trapezoidal_profile.json")
    test1_stats = calculate_statistics(test1_data)
    test1_validation = validate_test_1_trapezoidal(test1_stats, test1_data["samples"])
    print_test_results(
        "Test 1: Trapezoidal Motion Profile", test1_validation, test1_stats
    )

    # Test 2: Adaptive Control Load Step
    test2_data = load_test_data(results_dir / "demo_adaptive_load_step.json")
    test2_stats = calculate_statistics(test2_data)
    test2_validation = validate_test_2_adaptive(test2_stats, test2_data["samples"])
    print_test_results(
        "Test 2: Adaptive Control Load Step", test2_validation, test2_stats
    )

    # Test 3: High-Speed Motion
    test3_data = load_test_data(results_dir / "demo_high_speed_motion.json")
    test3_stats = calculate_statistics(test3_data)
    test3_validation = validate_test_3_high_speed(test3_stats, test3_data["samples"])
    print_test_results("Test 3: High-Speed Motion", test3_validation, test3_stats)

    # Overall verdict
    print(f"\n{'='*80}")
    print("## 🎯 OVERALL VERDICT")
    print(f"{'='*80}")

    all_issues = (
        test1_validation["issues"]
        + test2_validation["issues"]
        + test3_validation["issues"]
    )
    all_warnings = (
        test1_validation["warnings"]
        + test2_validation["warnings"]
        + test3_validation["warnings"]
    )

    if not all_issues:
        if len(all_warnings) <= 5:
            print("\n### General Assessment: ✅ EXCELLENT")
            print("All three tests pass with minimal warnings.")
        else:
            print("\n### General Assessment: ✅ GOOD")
            print("All tests pass but there are some warnings to review.")
    else:
        print("\n### General Assessment: ❌ NEEDS IMPROVEMENT")
        print(f"Found {len(all_issues)} critical issues across tests.")

    print("\n### Реалистичность: ", end="")
    if len(all_issues) == 0:
        print("✅ ДА - результаты реалистичны и соответствуют физике FOC")
    elif len(all_issues) <= 2:
        print("⚠️  ЧАСТИЧНО - есть небольшие несоответствия")
    else:
        print("❌ НЕТ - результаты требуют доработки")

    print("\n### 📝 Recommendations:")
    if len(all_issues) == 0 and len(all_warnings) <= 3:
        print("  1. ✅ FOC monitoring system working correctly")
        print("  2. ✅ Test data collection is accurate")
        print("  3. ✅ Visualization system produces valid reports")
        print("  4. System ready for integration testing")
    else:
        if all_issues:
            print("  1. Review and fix critical issues listed above")
        if len(all_warnings) > 5:
            print("  2. Investigate warnings to improve accuracy")
        print("  3. Consider tuning PI controller gains for better tracking")
        print("  4. Verify coolStep algorithm implementation")

    print("\n" + "=" * 80)


if __name__ == "__main__":
    main()
