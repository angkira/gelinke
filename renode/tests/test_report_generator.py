"""
Test Report Generator for Renode E2E Tests

Generates comprehensive FOC control plots and analysis reports from test data.
"""

import json
from pathlib import Path
from typing import List, Optional
import numpy as np
import matplotlib.pyplot as plt
from matplotlib.backends.backend_pdf import PdfPages
from matplotlib.gridspec import GridSpec
import pandas as pd


class FocTestReportGenerator:
    """
    Generates visual reports for FOC tests.

    Features:
    - FOC control loop visualization (position, velocity, current)
    - Motion profile tracking (target vs actual)
    - Phase analysis (PWM duty cycles, Clarke/Park transforms)
    - Performance metrics (tracking error, settling time, overshoot)
    - Adaptive control visualization (load estimation, coolStep, dcStep)
    - Health monitoring trends

    Usage:
        generator = FocTestReportGenerator("test_results/motion_test.json")
        generator.generate_pdf("test_results/motion_test_report.pdf")
    """

    def __init__(self, data_file: str):
        """
        Initialize report generator.

        Args:
            data_file: Path to test data JSON file
        """
        self.data_file = Path(data_file)

        # Load data
        with open(self.data_file, "r") as f:
            self.raw_data = json.load(f)

        self.metadata = self.raw_data.get("metadata", {})
        self.samples = self.raw_data.get("samples", [])
        self.statistics = self.raw_data.get("statistics", {})

        # Convert to pandas DataFrame for analysis
        self.df = pd.DataFrame(self.samples)

        print(f"Loaded {len(self.samples)} samples from {self.data_file.name}")

    def plot_motion_tracking(self, ax: plt.Axes):
        """Plot position and velocity tracking (target vs actual)."""
        if len(self.df) == 0:
            return

        # Create twin axis for velocity
        ax2 = ax.twinx()

        # Position
        ax.plot(
            self.df["timestamp"],
            self.df["position"],
            "b-",
            linewidth=2,
            label="Actual Position",
        )
        ax.plot(
            self.df["timestamp"],
            self.df["target_position"],
            "b--",
            linewidth=1.5,
            alpha=0.7,
            label="Target Position",
        )

        # Velocity
        ax2.plot(
            self.df["timestamp"],
            self.df["velocity"],
            "g-",
            linewidth=1.5,
            alpha=0.8,
            label="Actual Velocity",
        )
        ax2.plot(
            self.df["timestamp"],
            self.df["target_velocity"],
            "g--",
            linewidth=1,
            alpha=0.5,
            label="Target Velocity",
        )

        ax.set_xlabel("Time (s)", fontsize=11)
        ax.set_ylabel("Position (rad)", fontsize=11, color="b")
        ax2.set_ylabel("Velocity (rad/s)", fontsize=11, color="g")
        ax.tick_params(axis="y", labelcolor="b")
        ax2.tick_params(axis="y", labelcolor="g")
        ax.set_title("Motion Tracking Performance", fontsize=13, fontweight="bold")
        ax.grid(True, alpha=0.3)

        # Legends
        lines1, labels1 = ax.get_legend_handles_labels()
        lines2, labels2 = ax2.get_legend_handles_labels()
        ax.legend(lines1 + lines2, labels1 + labels2, loc="upper left", fontsize=9)

    def plot_tracking_error(self, ax: plt.Axes):
        """Plot position tracking error with statistics."""
        if len(self.df) == 0:
            return

        # Calculate error
        error = self.df["target_position"] - self.df["position"]
        error_rad = error
        error_deg = np.rad2deg(error_rad)

        # Plot error
        ax.plot(
            self.df["timestamp"],
            error_deg,
            "r-",
            linewidth=2,
            label="Tracking Error",
        )

        # Zero line
        ax.axhline(0, color="k", linestyle="--", linewidth=1, alpha=0.5)

        # ±1° band (typical requirement)
        ax.fill_between(
            self.df["timestamp"],
            -1.0,
            1.0,
            alpha=0.2,
            color="green",
            label="±1° Tolerance",
        )

        ax.set_xlabel("Time (s)", fontsize=11)
        ax.set_ylabel("Position Error (degrees)", fontsize=11)
        ax.set_title("Position Tracking Error", fontsize=13, fontweight="bold")
        ax.grid(True, alpha=0.3)
        ax.legend(loc="upper right", fontsize=9)

        # Add statistics box
        rms_error = np.sqrt(np.mean(error_deg**2))
        max_error = np.max(np.abs(error_deg))
        mean_error = np.mean(error_deg)

        stats_text = (
            f"RMS Error: {rms_error:.3f}°\n"
            f"Max Error: {max_error:.3f}°\n"
            f"Mean Error: {mean_error:.3f}°"
        )

        ax.text(
            0.02,
            0.98,
            stats_text,
            transform=ax.transAxes,
            fontsize=9,
            verticalalignment="top",
            bbox=dict(boxstyle="round", facecolor="wheat", alpha=0.8),
        )

    def plot_current_dq(self, ax: plt.Axes):
        """Plot d-q axis currents (FOC control)."""
        if len(self.df) == 0:
            return

        ax.plot(
            self.df["timestamp"],
            self.df["i_q"],
            "m-",
            linewidth=2,
            label="I_q (Torque)",
        )
        ax.plot(
            self.df["timestamp"],
            self.df["i_d"],
            "c-",
            linewidth=1.5,
            alpha=0.7,
            label="I_d (Flux)",
        )

        ax.set_xlabel("Time (s)", fontsize=11)
        ax.set_ylabel("Current (A)", fontsize=11)
        ax.set_title("FOC d-q Axis Currents", fontsize=13, fontweight="bold")
        ax.grid(True, alpha=0.3)
        ax.legend(loc="upper right", fontsize=9)

        # Calculate current magnitude
        i_mag = np.sqrt(self.df["i_q"] ** 2 + self.df["i_d"] ** 2)
        peak_current = np.max(i_mag)
        rms_current = np.sqrt(np.mean(i_mag**2))

        stats_text = (
            f"Peak I_q: {np.max(np.abs(self.df['i_q'])):.2f} A\n"
            f"RMS Current: {rms_current:.2f} A\n"
            f"Peak Magnitude: {peak_current:.2f} A"
        )

        ax.text(
            0.98,
            0.98,
            stats_text,
            transform=ax.transAxes,
            fontsize=9,
            verticalalignment="top",
            horizontalalignment="right",
            bbox=dict(boxstyle="round", facecolor="lightblue", alpha=0.8),
        )

    def plot_pwm_duty_cycles(self, ax: plt.Axes):
        """Plot 3-phase PWM duty cycles."""
        if len(self.df) == 0:
            return

        ax.plot(
            self.df["timestamp"],
            self.df["pwm_duty_a"],
            "r-",
            linewidth=1.5,
            alpha=0.8,
            label="Phase A",
        )
        ax.plot(
            self.df["timestamp"],
            self.df["pwm_duty_b"],
            "g-",
            linewidth=1.5,
            alpha=0.8,
            label="Phase B",
        )
        ax.plot(
            self.df["timestamp"],
            self.df["pwm_duty_c"],
            "b-",
            linewidth=1.5,
            alpha=0.8,
            label="Phase C",
        )

        ax.set_xlabel("Time (s)", fontsize=11)
        ax.set_ylabel("PWM Duty Cycle", fontsize=11)
        ax.set_title("3-Phase PWM Duty Cycles", fontsize=13, fontweight="bold")
        ax.set_ylim([0, 1])
        ax.grid(True, alpha=0.3)
        ax.legend(loc="upper right", fontsize=9)

    def plot_load_estimation(self, ax: plt.Axes):
        """Plot load estimation and temperature."""
        if len(self.df) == 0:
            return

        # Create twin axis for temperature
        ax2 = ax.twinx()

        # Load
        ax.plot(
            self.df["timestamp"],
            self.df["load_estimate"],
            "m-",
            linewidth=2,
            label="Load Estimate",
        )

        # Temperature
        ax2.plot(
            self.df["timestamp"],
            self.df["temperature"],
            "r-",
            linewidth=1.5,
            alpha=0.7,
            label="Temperature",
        )

        ax.set_xlabel("Time (s)", fontsize=11)
        ax.set_ylabel("Load (Nm)", fontsize=11, color="m")
        ax2.set_ylabel("Temperature (°C)", fontsize=11, color="r")
        ax.tick_params(axis="y", labelcolor="m")
        ax2.tick_params(axis="y", labelcolor="r")
        ax.set_title("Load Estimation & Temperature", fontsize=13, fontweight="bold")
        ax.grid(True, alpha=0.3)

        # Legends
        lines1, labels1 = ax.get_legend_handles_labels()
        lines2, labels2 = ax2.get_legend_handles_labels()
        ax.legend(lines1 + lines2, labels1 + labels2, loc="upper left", fontsize=9)

        # Statistics
        mean_load = np.mean(self.df["load_estimate"])
        max_load = np.max(np.abs(self.df["load_estimate"]))
        max_temp = np.max(self.df["temperature"])

        stats_text = (
            f"Mean Load: {mean_load:.3f} Nm\n"
            f"Peak Load: {max_load:.3f} Nm\n"
            f"Max Temp: {max_temp:.1f} °C"
        )

        ax.text(
            0.98,
            0.02,
            stats_text,
            transform=ax.transAxes,
            fontsize=9,
            verticalalignment="bottom",
            horizontalalignment="right",
            bbox=dict(boxstyle="round", facecolor="lightyellow", alpha=0.8),
        )

    def plot_health_score(self, ax: plt.Axes):
        """Plot health monitoring score over time."""
        if len(self.df) == 0:
            return

        ax.plot(
            self.df["timestamp"],
            self.df["health_score"],
            "g-",
            linewidth=2,
            label="Health Score",
        )

        # Threshold lines
        ax.axhline(
            80, color="y", linestyle="--", linewidth=1, alpha=0.7, label="Warning (80)"
        )
        ax.axhline(
            60, color="r", linestyle="--", linewidth=1, alpha=0.7, label="Critical (60)"
        )

        ax.set_xlabel("Time (s)", fontsize=11)
        ax.set_ylabel("Health Score", fontsize=11)
        ax.set_title("System Health Monitoring", fontsize=13, fontweight="bold")
        ax.set_ylim([0, 105])
        ax.grid(True, alpha=0.3)
        ax.legend(loc="lower left", fontsize=9)

        # Color regions
        ax.fill_between(
            self.df["timestamp"],
            100,
            80,
            alpha=0.1,
            color="green",
        )
        ax.fill_between(
            self.df["timestamp"],
            80,
            60,
            alpha=0.1,
            color="yellow",
        )
        ax.fill_between(
            self.df["timestamp"],
            60,
            0,
            alpha=0.1,
            color="red",
        )

    def plot_phase_diagram(self, ax: plt.Axes):
        """Plot position-velocity phase diagram."""
        if len(self.df) == 0:
            return

        # Color by time
        scatter = ax.scatter(
            self.df["position"],
            self.df["velocity"],
            c=self.df["timestamp"],
            cmap="viridis",
            s=10,
            alpha=0.6,
        )

        # Target trajectory
        ax.plot(
            self.df["target_position"],
            self.df["target_velocity"],
            "r--",
            linewidth=2,
            alpha=0.5,
            label="Target Trajectory",
        )

        ax.set_xlabel("Position (rad)", fontsize=11)
        ax.set_ylabel("Velocity (rad/s)", fontsize=11)
        ax.set_title(
            "Phase Diagram (Position-Velocity)", fontsize=13, fontweight="bold"
        )
        ax.grid(True, alpha=0.3)
        ax.legend(loc="upper right", fontsize=9)

        plt.colorbar(scatter, ax=ax, label="Time (s)")

    def generate_pdf(self, output_file: str):
        """
        Generate comprehensive PDF report.

        Args:
            output_file: Output PDF file path
        """
        print(f"\nGenerating FOC test report: {output_file}")

        with PdfPages(output_file) as pdf:
            # Page 1: Title and metadata
            fig = plt.figure(figsize=(11, 8.5))
            fig.suptitle(
                f"FOC Test Report: {self.metadata.get('test_name', 'Unknown')}",
                fontsize=18,
                fontweight="bold",
            )

            ax = fig.add_subplot(111)
            ax.axis("off")

            # Metadata table
            metadata_text = (
                f"Test Name: {self.metadata.get('test_name', 'N/A')}\n"
                f"Platform: {self.metadata.get('platform', 'N/A')}\n"
                f"Firmware: {self.metadata.get('firmware_version', 'N/A')}\n"
                f"Start Time: {self.metadata.get('start_time', 'N/A')}\n"
                f"\nSamples Collected: {self.statistics.get('sample_count', 0)}\n"
                f"Test Duration: {self.statistics.get('duration_s', 0):.3f} s\n"
            )

            ax.text(
                0.5,
                0.7,
                metadata_text,
                transform=ax.transAxes,
                fontsize=14,
                verticalalignment="top",
                horizontalalignment="center",
                bbox=dict(boxstyle="round,pad=1", facecolor="lightblue", alpha=0.3),
            )

            # Statistics summary
            if self.statistics:
                stats_text = "Performance Summary:\n\n"
                if "position" in self.statistics:
                    pos_stats = self.statistics["position"]
                    stats_text += f"Position Range: [{pos_stats['min']:.3f}, {pos_stats['max']:.3f}] rad\n"
                    stats_text += f"Position Std Dev: {pos_stats['std']:.3f} rad\n"

                if "velocity" in self.statistics:
                    vel_stats = self.statistics["velocity"]
                    stats_text += f"Max Velocity: {vel_stats['max']:.3f} rad/s\n"

                if "current_q" in self.statistics:
                    iq_stats = self.statistics["current_q"]
                    stats_text += f"Peak Current (I_q): {iq_stats['peak']:.2f} A\n"
                    stats_text += f"Mean Current (I_q): {iq_stats['mean']:.2f} A\n"

                ax.text(
                    0.5,
                    0.3,
                    stats_text,
                    transform=ax.transAxes,
                    fontsize=12,
                    verticalalignment="top",
                    horizontalalignment="center",
                    bbox=dict(
                        boxstyle="round,pad=1", facecolor="lightgreen", alpha=0.3
                    ),
                )

            pdf.savefig(fig, bbox_inches="tight")
            plt.close()

            # Page 2: Motion tracking
            fig, axes = plt.subplots(2, 1, figsize=(11, 8.5))
            self.plot_motion_tracking(axes[0])
            self.plot_tracking_error(axes[1])
            plt.tight_layout()
            pdf.savefig(fig, bbox_inches="tight")
            plt.close()

            # Page 3: FOC control
            fig, axes = plt.subplots(2, 1, figsize=(11, 8.5))
            self.plot_current_dq(axes[0])
            self.plot_pwm_duty_cycles(axes[1])
            plt.tight_layout()
            pdf.savefig(fig, bbox_inches="tight")
            plt.close()

            # Page 4: Adaptive control & diagnostics
            fig, axes = plt.subplots(2, 1, figsize=(11, 8.5))
            self.plot_load_estimation(axes[0])
            self.plot_health_score(axes[1])
            plt.tight_layout()
            pdf.savefig(fig, bbox_inches="tight")
            plt.close()

            # Page 5: Phase diagram
            fig, ax = plt.subplots(1, 1, figsize=(8, 8))
            self.plot_phase_diagram(ax)
            plt.tight_layout()
            pdf.savefig(fig, bbox_inches="tight")
            plt.close()

        print(f"✓ Report saved to: {output_file}")


def generate_test_suite_summary(test_results_dir: str, output_file: str):
    """
    Generate summary report for all tests in a directory.

    Args:
        test_results_dir: Directory containing test JSON files
        output_file: Output PDF file
    """
    results_path = Path(test_results_dir)
    json_files = list(results_path.glob("*.json"))

    # Filter out summary files
    json_files = [f for f in json_files if "summary" not in f.name.lower()]

    if not json_files:
        print(f"No test result files found in {test_results_dir}")
        return

    print(f"\nGenerating test suite summary for {len(json_files)} tests...")

    with PdfPages(output_file) as pdf:
        # Title page
        fig = plt.figure(figsize=(11, 8.5))
        fig.suptitle("FOC Test Suite Summary", fontsize=20, fontweight="bold")

        ax = fig.add_subplot(111)
        ax.axis("off")

        summary_text = f"Total Tests: {len(json_files)}\n\n"
        summary_text += "Tests:\n"

        for i, json_file in enumerate(json_files, 1):
            with open(json_file, "r") as f:
                data = json.load(f)

            metadata = data.get("metadata", {})
            stats = data.get("statistics", {})

            test_name = metadata.get("test_name", json_file.stem)
            sample_count = stats.get("sample_count", 0)
            duration = stats.get("duration_s", 0)

            summary_text += (
                f"{i}. {test_name}: {sample_count} samples, {duration:.2f}s\n"
            )

        ax.text(
            0.5,
            0.5,
            summary_text,
            transform=ax.transAxes,
            fontsize=12,
            verticalalignment="center",
            horizontalalignment="center",
            bbox=dict(boxstyle="round,pad=1", facecolor="lightblue", alpha=0.3),
        )

        pdf.savefig(fig, bbox_inches="tight")
        plt.close()

    print(f"✓ Test suite summary saved to: {output_file}")


# CLI interface
if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(
        description="Generate FOC test reports from Renode test data"
    )

    parser.add_argument(
        "--input",
        required=True,
        help="Input test data JSON file",
    )

    parser.add_argument(
        "--output",
        help="Output PDF report file (default: <input>_report.pdf)",
    )

    parser.add_argument(
        "--suite-summary",
        action="store_true",
        help="Generate summary for all tests in directory",
    )

    args = parser.parse_args()

    if args.suite_summary:
        output = args.output or "test_suite_summary.pdf"
        generate_test_suite_summary(args.input, output)
    else:
        # Single test report
        output = args.output or str(Path(args.input).with_suffix("")) + "_report.pdf"
        generator = FocTestReportGenerator(args.input)
        generator.generate_pdf(output)
