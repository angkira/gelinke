"""
Test Data Collector for Renode E2E Tests

Collects telemetry data from Renode mock peripherals and saves for post-analysis.
"""

import json
import time
from dataclasses import dataclass, asdict
from pathlib import Path
from typing import List, Optional
import numpy as np


@dataclass
class FocSnapshot:
    """Single FOC control loop snapshot."""

    timestamp: float  # Relative time (s)
    position: float  # Encoder position (rad)
    velocity: float  # Calculated velocity (rad/s)
    target_position: float  # Target from motion planner (rad)
    target_velocity: float  # Target velocity (rad/s)
    i_q: float  # Q-axis current (A)
    i_d: float  # D-axis current (A)
    load_estimate: float  # Load estimation (Nm)
    pwm_duty_a: float  # Phase A PWM duty (0-1)
    pwm_duty_b: float  # Phase B PWM duty (0-1)
    pwm_duty_c: float  # Phase C PWM duty (0-1)
    temperature: float  # Motor temperature (°C)
    health_score: float  # Health monitor score (0-100)


class TestDataCollector:
    """
    Collects FOC telemetry during Renode tests.

    Features:
    - Records data from mock peripherals
    - Supports multiple test cases
    - Exports to JSON/CSV
    - Compatible with analyze.py

    Usage:
        collector = TestDataCollector("motion_planning_test")
        collector.add_snapshot(FocSnapshot(...))
        collector.save("test_results/motion_planning.json")
    """

    def __init__(self, test_name: str):
        """
        Initialize collector for a test case.

        Args:
            test_name: Name of the test case
        """
        self.test_name = test_name
        self.snapshots: List[FocSnapshot] = []
        self.start_time = time.time()
        self.metadata = {
            "test_name": test_name,
            "start_time": time.strftime("%Y-%m-%d %H:%M:%S"),
            "platform": "Renode STM32G431CB",
            "firmware_version": "iRPC v2.0",
        }

    def add_snapshot(self, snapshot: FocSnapshot):
        """Add telemetry snapshot to collection."""
        self.snapshots.append(snapshot)

    def add_from_peripherals(
        self,
        encoder_position: float,
        encoder_velocity: float,
        adc_i_q: float,
        adc_i_d: float,
        motor_pwm_a: float,
        motor_pwm_b: float,
        motor_pwm_c: float,
        target_position: float = 0.0,
        target_velocity: float = 0.0,
        load_estimate: float = 0.0,
        temperature: float = 25.0,
        health_score: float = 100.0,
    ):
        """
        Add snapshot from Renode mock peripheral values.

        Args:
            encoder_position: From AS5047P encoder mock
            encoder_velocity: Calculated from encoder
            adc_i_q: From current sense ADC (Q-axis)
            adc_i_d: From current sense ADC (D-axis)
            motor_pwm_a: From motor simulator (phase A duty)
            motor_pwm_b: From motor simulator (phase B duty)
            motor_pwm_c: From motor simulator (phase C duty)
            target_position: From motion planner
            target_velocity: From motion planner
            load_estimate: From adaptive controller
            temperature: From motor simulator
            health_score: From health monitor
        """
        snapshot = FocSnapshot(
            timestamp=time.time() - self.start_time,
            position=encoder_position,
            velocity=encoder_velocity,
            target_position=target_position,
            target_velocity=target_velocity,
            i_q=adc_i_q,
            i_d=adc_i_d,
            load_estimate=load_estimate,
            pwm_duty_a=motor_pwm_a,
            pwm_duty_b=motor_pwm_b,
            pwm_duty_c=motor_pwm_c,
            temperature=temperature,
            health_score=health_score,
        )
        self.add_snapshot(snapshot)

    def save_json(self, filepath: str):
        """
        Save collected data to JSON.

        Args:
            filepath: Output JSON file path
        """
        output = {
            "metadata": self.metadata,
            "samples": [asdict(s) for s in self.snapshots],
            "statistics": self._calculate_statistics(),
        }

        Path(filepath).parent.mkdir(parents=True, exist_ok=True)

        with open(filepath, "w") as f:
            json.dump(output, f, indent=2)

        print(f"✓ Saved {len(self.snapshots)} samples to {filepath}")

    def save_csv(self, filepath: str):
        """
        Save collected data to CSV (compatible with analyze.py).

        Args:
            filepath: Output CSV file path
        """
        import csv

        Path(filepath).parent.mkdir(parents=True, exist_ok=True)

        with open(filepath, "w", newline="") as f:
            if not self.snapshots:
                return

            fieldnames = list(asdict(self.snapshots[0]).keys())
            writer = csv.DictWriter(f, fieldnames=fieldnames)
            writer.writeheader()

            for snapshot in self.snapshots:
                writer.writerow(asdict(snapshot))

        print(f"✓ Saved {len(self.snapshots)} samples to {filepath}")

    def save_pandas_csv(self, filepath: str):
        """
        Save in pandas-compatible format for analyze.py.

        Args:
            filepath: Output CSV file path
        """
        import csv

        Path(filepath).parent.mkdir(parents=True, exist_ok=True)

        # Format for analyze.py: time, position, velocity, load, temperature
        with open(filepath, "w", newline="") as f:
            writer = csv.writer(f)
            writer.writerow(["time", "position", "velocity", "load", "temperature"])

            for snapshot in self.snapshots:
                writer.writerow(
                    [
                        snapshot.timestamp,
                        snapshot.position,
                        snapshot.velocity,
                        snapshot.load_estimate,
                        snapshot.temperature,
                    ]
                )

        print(
            f"✓ Saved {len(self.snapshots)} samples (analyze.py format) to {filepath}"
        )

    def _calculate_statistics(self) -> dict:
        """Calculate statistical summary of collected data."""
        if not self.snapshots:
            return {}

        positions = np.array([s.position for s in self.snapshots])
        velocities = np.array([s.velocity for s in self.snapshots])
        currents_q = np.array([s.i_q for s in self.snapshots])
        loads = np.array([s.load_estimate for s in self.snapshots])

        return {
            "sample_count": len(self.snapshots),
            "duration_s": self.snapshots[-1].timestamp if self.snapshots else 0.0,
            "position": {
                "mean": float(np.mean(positions)),
                "std": float(np.std(positions)),
                "min": float(np.min(positions)),
                "max": float(np.max(positions)),
            },
            "velocity": {
                "mean": float(np.mean(velocities)),
                "std": float(np.std(velocities)),
                "min": float(np.min(velocities)),
                "max": float(np.max(velocities)),
            },
            "current_q": {
                "mean": float(np.mean(currents_q)),
                "std": float(np.std(currents_q)),
                "peak": float(np.max(np.abs(currents_q))),
            },
            "load_estimate": {
                "mean": float(np.mean(loads)),
                "std": float(np.std(loads)),
                "max": float(np.max(np.abs(loads))),
            },
        }

    def get_statistics(self) -> dict:
        """Get statistics without saving."""
        return self._calculate_statistics()


class MultiTestCollector:
    """
    Manages data collection for multiple test cases.

    Usage:
        collector = MultiTestCollector("test_results/")

        # Test 1
        test1 = collector.start_test("trapezoidal_profile")
        test1.add_snapshot(...)
        collector.finish_test()

        # Test 2
        test2 = collector.start_test("s_curve_profile")
        test2.add_snapshot(...)
        collector.finish_test()

        # Generate report
        collector.generate_report()
    """

    def __init__(self, output_dir: str = "test_results"):
        """
        Initialize multi-test collector.

        Args:
            output_dir: Directory for test results
        """
        self.output_dir = Path(output_dir)
        self.output_dir.mkdir(parents=True, exist_ok=True)
        self.collectors: List[TestDataCollector] = []
        self.current_collector: Optional[TestDataCollector] = None

    def start_test(self, test_name: str) -> TestDataCollector:
        """
        Start collecting data for a new test case.

        Args:
            test_name: Name of the test case

        Returns:
            TestDataCollector for this test
        """
        self.current_collector = TestDataCollector(test_name)
        self.collectors.append(self.current_collector)
        return self.current_collector

    def finish_test(self):
        """Finish current test and save data."""
        if self.current_collector:
            # Save in multiple formats
            test_name = self.current_collector.test_name
            self.current_collector.save_json(str(self.output_dir / f"{test_name}.json"))
            self.current_collector.save_csv(
                str(self.output_dir / f"{test_name}_full.csv")
            )
            self.current_collector.save_pandas_csv(
                str(self.output_dir / f"{test_name}.csv")
            )
            self.current_collector = None

    def generate_summary(self) -> dict:
        """Generate summary of all test results."""
        summary = {
            "total_tests": len(self.collectors),
            "output_directory": str(self.output_dir),
            "tests": [],
        }

        for collector in self.collectors:
            stats = collector.get_statistics()
            summary["tests"].append(
                {
                    "name": collector.test_name,
                    "samples": stats.get("sample_count", 0),
                    "duration_s": stats.get("duration_s", 0.0),
                    "statistics": stats,
                }
            )

        return summary

    def save_summary(self, filename: str = "test_summary.json"):
        """Save summary to JSON file."""
        summary = self.generate_summary()
        filepath = self.output_dir / filename

        with open(filepath, "w") as f:
            json.dump(summary, f, indent=2)

        print(f"\n{'='*60}")
        print(f"Test Summary: {len(self.collectors)} tests completed")
        print(f"Results saved to: {self.output_dir}")
        print(f"{'='*60}\n")

        return str(filepath)


# Example usage for Robot Framework
if __name__ == "__main__":
    # Simulate a test
    collector = TestDataCollector("example_motion_test")

    # Simulate 1 second of FOC loop at 10 kHz
    dt = 0.0001  # 100 µs (10 kHz)
    target = 1.57  # 90 degrees

    for i in range(10000):
        t = i * dt

        # Simulate motion towards target
        position = target * (1 - np.exp(-5 * t))
        velocity = target * 5 * np.exp(-5 * t)

        # Simulate PI controller
        error = target - position
        i_q = 0.5 * error + 0.1 * velocity

        # Simulate load
        load = 0.1 * i_q

        collector.add_from_peripherals(
            encoder_position=position,
            encoder_velocity=velocity,
            adc_i_q=i_q,
            adc_i_d=0.0,
            motor_pwm_a=0.5,
            motor_pwm_b=0.5,
            motor_pwm_c=0.5,
            target_position=target,
            target_velocity=0.0,
            load_estimate=load,
        )

    # Save data
    collector.save_json("test_results/example.json")
    collector.save_pandas_csv("test_results/example.csv")

    print("\nStatistics:")
    stats = collector.get_statistics()
    print(f"  Samples: {stats['sample_count']}")
    print(f"  Duration: {stats['duration_s']:.3f} s")
    print(f"  Position mean: {stats['position']['mean']:.3f} rad")
    print(f"  Velocity peak: {stats['velocity']['max']:.3f} rad/s")
