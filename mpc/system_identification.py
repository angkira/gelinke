#!/usr/bin/env python3
"""
System Identification for MPC Controller

This script collects data from the motor system and identifies the state-space model:
  State: x = [position, velocity, acceleration]
  Input: u = jerk
  Model: x[k+1] = A*x[k] + B*u[k]

Usage:
  1. Run test sequences on real hardware
  2. Fit model parameters
  3. Validate model accuracy
  4. Generate discrete-time state-space matrices for MPC

Author: Generated for MPC implementation
Date: 2025-10-08
"""

import numpy as np
import json
from pathlib import Path
from typing import Dict, Tuple
import matplotlib.pyplot as plt
from scipy.optimize import minimize, curve_fit
from scipy.signal import cont2discrete
from dataclasses import dataclass, asdict


@dataclass
class MotorParameters:
    """Physical motor parameters identified from tests."""
    J: float            # Inertia (kg·m²)
    B: float            # Viscous friction coefficient (Nm/(rad/s))
    K_t: float          # Torque constant (Nm/A)
    tau_current: float  # Current loop time constant (s)
    tau_delay: float    # System delay (s)

    # Optional: Coulomb friction
    friction_coulomb: float = 0.0  # Nm

    def to_dict(self):
        """Convert to dictionary for JSON serialization."""
        return asdict(self)

    @classmethod
    def from_dict(cls, d: Dict):
        """Load from dictionary."""
        return cls(**d)


class SystemIdentification:
    """System identification for motor dynamics."""

    def __init__(self, sampling_time: float = 0.0001):
        """
        Initialize system identification.

        Args:
            sampling_time: Control loop sampling time (s)
        """
        self.dt = sampling_time
        self.params = None
        self.A = None
        self.B = None

    def generate_test_sequences(self, output_file: str = "test_sequences.json"):
        """
        Generate test input sequences for system identification.

        Args:
            output_file: Path to save test sequences

        Returns:
            Dictionary of test sequences
        """
        sequences = {}

        # Test 1: Step response (various amplitudes)
        for amp in [0.5, 1.0, 1.5, 2.0]:
            sequences[f"step_response_{amp}rad"] = {
                "type": "step",
                "amplitude": amp,
                "duration": 1.0,
                "description": f"Step input to {amp} rad"
            }

        # Test 2: Impulse response
        sequences["impulse_response"] = {
            "type": "impulse",
            "amplitude": 2.0,
            "width": 0.05,  # 50ms pulse
            "duration": 1.0,
            "description": "Impulse input to measure natural dynamics"
        }

        # Test 3: Frequency sweep (chirp)
        sequences["frequency_sweep"] = {
            "type": "chirp",
            "freq_start": 0.1,  # Hz
            "freq_end": 50.0,   # Hz
            "amplitude": 1.0,
            "duration": 5.0,
            "description": "Frequency sweep from 0.1 to 50 Hz"
        }

        # Test 4: Pseudo-random binary sequence (PRBS)
        for level in [3, 5, 7]:
            sequences[f"prbs_level{level}"] = {
                "type": "prbs",
                "level": level,
                "amplitude": 1.0,
                "duration": 2.0,
                "description": f"PRBS excitation (level {level})"
            }

        # Test 5: Ramp response (constant acceleration)
        sequences["ramp_response"] = {
            "type": "ramp",
            "acceleration": 5.0,  # rad/s²
            "duration": 0.5,
            "description": "Ramp input to measure acceleration response"
        }

        # Save sequences
        with open(output_file, 'w') as f:
            json.dump(sequences, f, indent=2)

        print(f"Generated {len(sequences)} test sequences")
        print(f"Saved to: {output_file}")
        print("\nTest sequence summary:")
        for name, seq in sequences.items():
            print(f"  - {name}: {seq['description']}")

        return sequences

    def load_test_data(self, data_file: str) -> Dict[str, np.ndarray]:
        """
        Load test data from CSV or JSON file.

        Expected format:
          time, position, velocity, acceleration, current_q, voltage_q

        Args:
            data_file: Path to data file

        Returns:
            Dictionary with time series data
        """
        if data_file.endswith('.json'):
            with open(data_file, 'r') as f:
                data = json.load(f)
            # Convert lists to numpy arrays
            return {k: np.array(v) for k, v in data.items()}

        elif data_file.endswith('.csv'):
            # Load CSV
            import pandas as pd
            df = pd.read_csv(data_file)
            return {col: df[col].values for col in df.columns}

        else:
            raise ValueError(f"Unsupported file format: {data_file}")

    def fit_motor_model(self, data: Dict[str, np.ndarray]) -> MotorParameters:
        """
        Fit motor parameters from test data.

        Uses optimization to find:
          J: Inertia
          B: Viscous friction
          K_t: Torque constant
          tau: Time constant

        Args:
            data: Test data dictionary with keys:
                  'time', 'position', 'velocity', 'acceleration', 'current_q'

        Returns:
            MotorParameters with fitted values
        """
        print("Fitting motor model...")

        # Extract data
        t = data['time']
        pos = data['position']
        vel = data['velocity']
        acc = data.get('acceleration', np.gradient(vel, t))  # Compute if not provided
        i_q = data['current_q']

        # Motor dynamics: J*a + B*v = K_t * i_q
        # Rearrange: a = (K_t/J)*i_q - (B/J)*v

        def motor_dynamics(params, t_data, i_q_data, v_data):
            """Simulate motor dynamics with given parameters."""
            J, B, K_t, tau = params

            # Simple Euler integration
            v_sim = np.zeros_like(t_data)
            a_sim = np.zeros_like(t_data)

            for i in range(1, len(t_data)):
                dt = t_data[i] - t_data[i-1]

                # Motor equation with time constant
                torque = K_t * i_q_data[i]
                friction = B * v_sim[i-1]

                # Acceleration
                a_sim[i] = (torque - friction) / J

                # Apply time constant (low-pass filter on acceleration)
                alpha = dt / (tau + dt)
                a_sim[i] = alpha * a_sim[i] + (1 - alpha) * a_sim[i-1]

                # Integrate to velocity
                v_sim[i] = v_sim[i-1] + a_sim[i] * dt

            return v_sim

        def objective(params):
            """Objective function: minimize velocity tracking error."""
            if np.any(np.array(params) <= 0):  # All params must be positive
                return 1e10

            v_sim = motor_dynamics(params, t, i_q, vel)
            error = np.mean((vel - v_sim)**2)
            return error

        # Initial guess (reasonable values for small motor)
        J_init = 0.001      # kg·m²
        B_init = 0.01       # Nm/(rad/s)
        K_t_init = 0.15     # Nm/A (typical)
        tau_init = 0.005    # 5ms time constant

        x0 = [J_init, B_init, K_t_init, tau_init]

        # Bounds (physical constraints)
        bounds = [
            (1e-5, 0.1),    # J: 0.01 g·m² to 100 g·m²
            (1e-4, 1.0),    # B: small to moderate friction
            (0.01, 1.0),    # K_t: typical range
            (1e-4, 0.05),   # tau: 0.1ms to 50ms
        ]

        # Optimize
        print("  Running optimization...")
        result = minimize(objective, x0, method='L-BFGS-B', bounds=bounds)

        if not result.success:
            print(f"  Warning: Optimization did not converge: {result.message}")

        J_opt, B_opt, K_t_opt, tau_opt = result.x

        # Estimate delay (from cross-correlation)
        tau_delay = self._estimate_delay(t, i_q, vel)

        params = MotorParameters(
            J=J_opt,
            B=B_opt,
            K_t=K_t_opt,
            tau_current=tau_opt,
            tau_delay=tau_delay
        )

        print(f"\n✅ Fitted parameters:")
        print(f"  Inertia (J):        {J_opt*1e6:.2f} g·m²")
        print(f"  Friction (B):       {B_opt*1e3:.2f} mNm/(rad/s)")
        print(f"  Torque const (K_t): {K_t_opt:.4f} Nm/A")
        print(f"  Time constant:      {tau_opt*1e3:.2f} ms")
        print(f"  System delay:       {tau_delay*1e3:.2f} ms")
        print(f"  Fit error (RMS):    {np.sqrt(result.fun):.6f} rad/s")

        self.params = params
        return params

    def _estimate_delay(self, t, input_signal, output_signal) -> float:
        """Estimate system delay using cross-correlation."""
        # Cross-correlation
        corr = np.correlate(output_signal - np.mean(output_signal),
                           input_signal - np.mean(input_signal),
                           mode='full')

        # Find peak
        delay_samples = np.argmax(corr) - len(input_signal) + 1

        # Convert to time
        dt = np.mean(np.diff(t))
        delay_time = abs(delay_samples * dt)

        # Clamp to reasonable range (0-10ms)
        return np.clip(delay_time, 0, 0.01)

    def validate_model(self, data: Dict[str, np.ndarray],
                      params: MotorParameters) -> Dict[str, float]:
        """
        Validate fitted model against test data.

        Args:
            data: Validation data (different from training data)
            params: Fitted motor parameters

        Returns:
            Dictionary with validation metrics
        """
        print("\nValidating model...")

        t = data['time']
        vel_actual = data['velocity']
        i_q = data['current_q']

        # Simulate model
        vel_pred = self._simulate_model(t, i_q, params)

        # Compute metrics
        error = vel_actual - vel_pred
        rms_error = np.sqrt(np.mean(error**2))
        max_error = np.max(np.abs(error))

        # Normalized error (percentage of signal range)
        signal_range = np.max(vel_actual) - np.min(vel_actual)
        relative_error = rms_error / signal_range * 100 if signal_range > 0 else 0

        # Correlation coefficient (R²)
        ss_res = np.sum(error**2)
        ss_tot = np.sum((vel_actual - np.mean(vel_actual))**2)
        r_squared = 1 - ss_res / ss_tot if ss_tot > 0 else 0

        metrics = {
            "rms_error": rms_error,
            "max_error": max_error,
            "relative_error_percent": relative_error,
            "r_squared": r_squared
        }

        print(f"\n📊 Validation metrics:")
        print(f"  RMS error:      {rms_error:.6f} rad/s")
        print(f"  Max error:      {max_error:.6f} rad/s")
        print(f"  Relative error: {relative_error:.2f}%")
        print(f"  R² score:       {r_squared:.4f}")

        if relative_error < 5.0 and r_squared > 0.9:
            print("  ✅ Model validation PASSED (excellent fit)")
        elif relative_error < 10.0 and r_squared > 0.8:
            print("  ⚠️  Model validation marginal (acceptable fit)")
        else:
            print("  ❌ Model validation FAILED (poor fit)")
            print("     → Re-run identification with more data or check for issues")

        return metrics

    def _simulate_model(self, t, i_q, params: MotorParameters) -> np.ndarray:
        """Simulate motor model forward in time."""
        vel = np.zeros_like(t)
        acc = 0.0

        for i in range(1, len(t)):
            dt = t[i] - t[i-1]

            # Motor dynamics
            torque = params.K_t * i_q[i]
            friction = params.B * vel[i-1]
            acc_new = (torque - friction) / params.J

            # Apply time constant
            alpha = dt / (params.tau_current + dt)
            acc = alpha * acc_new + (1 - alpha) * acc

            # Integrate
            vel[i] = vel[i-1] + acc * dt

        return vel

    def generate_statespace_matrices(self, params: MotorParameters) -> Tuple[np.ndarray, np.ndarray]:
        """
        Generate discrete-time state-space matrices for MPC.

        State: x = [position, velocity, acceleration]
        Input: u = jerk

        Continuous-time:
          dx/dt = A_c * x + B_c * u

        Discrete-time:
          x[k+1] = A * x[k] + B * u[k]

        Args:
            params: Fitted motor parameters

        Returns:
            Tuple of (A, B) matrices
        """
        print("\nGenerating state-space matrices...")

        # Continuous-time system
        # State: [pos, vel, acc]
        # Dynamics:
        #   d(pos)/dt = vel
        #   d(vel)/dt = acc
        #   d(acc)/dt = (1/tau) * (K_t*i_q/J - B*vel/J - acc)
        #
        # Simplification for MPC: treat acc as controlled variable
        #   d(acc)/dt = jerk = u

        A_c = np.array([
            [0, 1, 0],
            [0, 0, 1],
            [0, 0, -1/params.tau_current]  # Acceleration decay
        ])

        B_c = np.array([
            [0],
            [0],
            [1/params.tau_current]  # Jerk input
        ])

        # Discretize using exact method (matrix exponential)
        A_d, B_d, _, _, _ = cont2discrete((A_c, B_c, np.eye(3), np.zeros((3,1))), self.dt, method='zoh')

        # For simple integrator chain, can also use direct formula:
        # (but exact discretization is more accurate)
        dt = self.dt
        A_simple = np.array([
            [1, dt, dt**2/2],
            [0, 1,  dt     ],
            [0, 0,  1      ]
        ])

        B_simple = np.array([
            [dt**3/6],
            [dt**2/2],
            [dt     ]
        ])

        # Use exact discretization
        A = A_d
        B = B_d

        print(f"  Sampling time: {self.dt*1e3:.2f} ms")
        print(f"\n  A matrix (3x3):")
        print(f"    {A[0]}")
        print(f"    {A[1]}")
        print(f"    {A[2]}")
        print(f"\n  B matrix (3x1):")
        print(f"    {B.flatten()}")

        self.A = A
        self.B = B

        return A, B

    def save_model(self, output_file: str = "motor_model.json"):
        """
        Save identified model to JSON file.

        Args:
            output_file: Path to save model
        """
        if self.params is None:
            raise ValueError("No model fitted yet. Run fit_motor_model() first.")

        model_data = {
            "parameters": self.params.to_dict(),
            "sampling_time": self.dt,
            "A_matrix": self.A.tolist() if self.A is not None else None,
            "B_matrix": self.B.tolist() if self.B is not None else None,
        }

        with open(output_file, 'w') as f:
            json.dump(model_data, f, indent=2)

        print(f"\n✅ Model saved to: {output_file}")

    def load_model(self, model_file: str):
        """
        Load identified model from JSON file.

        Args:
            model_file: Path to model file
        """
        with open(model_file, 'r') as f:
            model_data = json.load(f)

        self.params = MotorParameters.from_dict(model_data["parameters"])
        self.dt = model_data["sampling_time"]
        self.A = np.array(model_data["A_matrix"]) if model_data["A_matrix"] else None
        self.B = np.array(model_data["B_matrix"]) if model_data["B_matrix"] else None

        print(f"✅ Model loaded from: {model_file}")

    def plot_validation(self, data: Dict[str, np.ndarray],
                       params: MotorParameters,
                       save_path: str = "validation_plot.png"):
        """
        Plot validation results.

        Args:
            data: Test data
            params: Fitted parameters
            save_path: Path to save plot
        """
        t = data['time']
        vel_actual = data['velocity']
        i_q = data['current_q']

        vel_pred = self._simulate_model(t, i_q, params)
        error = vel_actual - vel_pred

        fig, axes = plt.subplots(3, 1, figsize=(10, 8))

        # Velocity comparison
        axes[0].plot(t, vel_actual, 'b-', label='Actual', linewidth=2)
        axes[0].plot(t, vel_pred, 'r--', label='Model', linewidth=1.5)
        axes[0].set_ylabel('Velocity (rad/s)')
        axes[0].legend()
        axes[0].grid(True, alpha=0.3)
        axes[0].set_title('Model Validation')

        # Error
        axes[1].plot(t, error, 'r-', linewidth=1)
        axes[1].set_ylabel('Error (rad/s)')
        axes[1].axhline(0, color='k', linestyle='--', alpha=0.3)
        axes[1].grid(True, alpha=0.3)

        # Input current
        axes[2].plot(t, i_q, 'g-', linewidth=1)
        axes[2].set_ylabel('Current (A)')
        axes[2].set_xlabel('Time (s)')
        axes[2].grid(True, alpha=0.3)

        plt.tight_layout()
        plt.savefig(save_path, dpi=150)
        print(f"  Plot saved to: {save_path}")


def main():
    """Main system identification workflow."""
    print("=" * 80)
    print("SYSTEM IDENTIFICATION FOR MPC CONTROLLER")
    print("=" * 80)

    # Initialize
    sysid = SystemIdentification(sampling_time=0.0001)  # 10 kHz

    # Step 1: Generate test sequences
    print("\nStep 1: Generate test sequences")
    sequences = sysid.generate_test_sequences("test_sequences.json")
    print("\n  → Run these sequences on your hardware and collect data")
    print("  → Save data as CSV with columns: time, position, velocity, acceleration, current_q")

    # For now, create synthetic data for testing
    print("\n  (Creating synthetic test data for demonstration...)")

    # Synthetic data
    t = np.linspace(0, 1, 10000)
    i_q = np.sin(2 * np.pi * 2 * t) + 0.5 * np.sin(2 * np.pi * 5 * t)

    # True parameters (for synthetic data)
    J_true = 0.002
    B_true = 0.02
    K_t_true = 0.15

    vel = np.zeros_like(t)
    for i in range(1, len(t)):
        dt = t[i] - t[i-1]
        torque = K_t_true * i_q[i]
        friction = B_true * vel[i-1]
        acc = (torque - friction) / J_true
        vel[i] = vel[i-1] + acc * dt

    pos = np.cumsum(vel) * (t[1] - t[0])

    # Add noise
    vel += np.random.normal(0, 0.001, len(vel))

    data = {
        'time': t,
        'position': pos,
        'velocity': vel,
        'current_q': i_q
    }

    # Step 2: Fit model
    print("\n\nStep 2: Fit motor model")
    params = sysid.fit_motor_model(data)

    # Step 3: Validate
    print("\n\nStep 3: Validate model")
    metrics = sysid.validate_model(data, params)

    # Step 4: Generate state-space matrices
    print("\n\nStep 4: Generate state-space matrices")
    A, B = sysid.generate_statespace_matrices(params)

    # Step 5: Save
    print("\n\nStep 5: Save model")
    sysid.save_model("motor_model.json")

    # Step 6: Plot
    print("\n\nStep 6: Generate validation plot")
    sysid.plot_validation(data, params, "validation_plot.png")

    print("\n" + "=" * 80)
    print("✅ SYSTEM IDENTIFICATION COMPLETE")
    print("=" * 80)
    print("\nNext steps:")
    print("  1. Review motor_model.json")
    print("  2. Check validation_plot.png")
    print("  3. If validation passed, proceed to MPC implementation")
    print("  4. If validation failed, collect more/better data and re-run")


if __name__ == "__main__":
    main()
