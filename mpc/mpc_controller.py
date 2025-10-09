#!/usr/bin/env python3
"""
Model Predictive Control (MPC) Controller - Python Prototype

This implements MPC for motor position control using CVXPY/OSQP.

MPC formulation:
  State: x = [position, velocity, acceleration]
  Input: u = jerk

  minimize: Σ(Q*(x-x_ref)² + R*u²)
  subject to:
    x[k+1] = A*x[k] + B*u[k]  (dynamics)
    |velocity| ≤ v_max
    |accel| ≤ a_max
    |jerk| ≤ j_max

Author: Generated for MPC implementation
Date: 2025-10-08
"""

import numpy as np
import cvxpy as cp
from typing import Tuple, Dict
import time


class MPCController:
    """Model Predictive Controller for motor position control."""

    def __init__(
        self,
        A: np.ndarray,
        B: np.ndarray,
        N: int = 50,
        dt: float = 0.0001,
        Q: np.ndarray | None = None,
        R: float = 0.01,
        v_max: float = 2.0,
        a_max: float = 5.0,
        j_max: float = 100.0,
    ):
        """
        Initialize MPC controller.

        Args:
            A: State transition matrix (3x3)
            B: Input matrix (3x1)
            N: Prediction horizon (timesteps)
            dt: Sampling time (s)
            Q: State cost matrix (3x3 or 3,), default diag([100, 10, 1])
            R: Input cost (scalar)
            v_max: Maximum velocity (rad/s)
            a_max: Maximum acceleration (rad/s²)
            j_max: Maximum jerk (rad/s³)
        """
        self.A = A
        self.B = B
        self.N = N
        self.dt = dt

        # Cost matrices
        if Q is None:
            self.Q = np.diag([100.0, 10.0, 1.0])  # Position, velocity, accel
        elif len(Q.shape) == 1:
            self.Q = np.diag(Q)
        else:
            self.Q = Q

        self.R = R

        # Constraints
        self.v_max = v_max
        self.a_max = a_max
        self.j_max = j_max

        # Problem dimensions
        self.n_states = 3
        self.n_inputs = 1

        # Setup optimization problem
        self._setup_optimization()

        # Statistics
        self.solve_times = []
        self.solve_status = []

    def _setup_optimization(self):
        """Setup CVXPY optimization problem (define once, solve repeatedly)."""
        n = self.n_states
        m = self.n_inputs
        N = self.N

        # Decision variables
        self.x = cp.Variable((n, N + 1))  # States over horizon
        self.u = cp.Variable((m, N))  # Inputs over horizon

        # Parameters (updated each solve)
        self.x_init = cp.Parameter(n)  # Initial state
        self.x_ref = cp.Parameter((n, N + 1))  # Reference trajectory

        # Cost function
        cost = 0
        for k in range(N):
            # Tracking error cost
            error = self.x[:, k] - self.x_ref[:, k]
            cost += cp.quad_form(error, self.Q)

            # Control effort cost
            cost += self.R * cp.square(self.u[0, k])

        # Terminal cost
        error_N = self.x[:, N] - self.x_ref[:, N]
        cost += cp.quad_form(error_N, self.Q)

        # Constraints
        constraints = []

        # Initial condition
        constraints.append(self.x[:, 0] == self.x_init)

        # Dynamics constraints
        for k in range(N):
            constraints.append(self.x[:, k + 1] == self.A @ self.x[:, k] + self.B @ self.u[:, k])

        # State constraints (velocity and acceleration bounds)
        for k in range(N + 1):
            constraints.append(self.x[1, k] <= self.v_max)  # vel <= v_max
            constraints.append(self.x[1, k] >= -self.v_max)  # vel >= -v_max
            constraints.append(self.x[2, k] <= self.a_max)  # accel <= a_max
            constraints.append(self.x[2, k] >= -self.a_max)  # accel >= -a_max

        # Input constraints (jerk bounds)
        for k in range(N):
            constraints.append(self.u[0, k] <= self.j_max)  # jerk <= j_max
            constraints.append(self.u[0, k] >= -self.j_max)  # jerk >= -j_max

        # Create problem
        self.problem = cp.Problem(cp.Minimize(cost), constraints)

    def solve(
        self, x_current: np.ndarray, x_ref_trajectory: np.ndarray, verbose: bool = False
    ) -> Tuple[float, Dict]:
        """
        Solve MPC optimization problem.

        Args:
            x_current: Current state [pos, vel, acc] (3,)
            x_ref_trajectory: Reference trajectory over horizon (3, N+1)
            verbose: Print solver output

        Returns:
            Tuple of (optimal_jerk, info_dict)
            where info_dict contains:
                - status: Solver status
                - solve_time: Time to solve (s)
                - cost: Optimal cost value
                - x_predicted: Predicted state trajectory (3, N+1)
                - u_optimal: Optimal input sequence (N,)
        """
        start_time = time.perf_counter()

        # Update parameters
        self.x_init.value = x_current
        self.x_ref.value = x_ref_trajectory

        # Solve optimization
        try:
            self.problem.solve(solver=cp.OSQP, warm_start=True, verbose=verbose, eps_abs=1e-4, eps_rel=1e-4)
        except Exception as e:
            print(f"MPC solve failed: {e}")
            return 0.0, {
                "status": "error",
                "solve_time": time.perf_counter() - start_time,
                "cost": float("inf"),
            }

        solve_time = time.perf_counter() - start_time

        # Check solution status
        if self.problem.status not in [cp.OPTIMAL, cp.OPTIMAL_INACCURATE]:
            if verbose:
                print(f"Warning: MPC solver status = {self.problem.status}")

            # Return zero jerk as fallback
            return 0.0, {
                "status": self.problem.status,
                "solve_time": solve_time,
                "cost": float("inf"),
            }

        # Extract solution
        u_optimal_seq = self.u.value.flatten() if self.u.value is not None else np.zeros(self.N)
        x_predicted = self.x.value if self.x.value is not None else None

        # Return first control input (receding horizon principle)
        u_opt = u_optimal_seq[0]

        # Statistics
        self.solve_times.append(solve_time)
        self.solve_status.append(self.problem.status)

        info = {
            "status": self.problem.status,
            "solve_time": solve_time,
            "cost": self.problem.value,
            "x_predicted": x_predicted,
            "u_optimal": u_optimal_seq,
        }

        return u_opt, info

    def get_statistics(self) -> Dict:
        """Get solver statistics."""
        if not self.solve_times:
            return {"count": 0}

        return {
            "count": len(self.solve_times),
            "mean_solve_time": np.mean(self.solve_times) * 1e6,  # µs
            "max_solve_time": np.max(self.solve_times) * 1e6,  # µs
            "min_solve_time": np.min(self.solve_times) * 1e6,  # µs
            "std_solve_time": np.std(self.solve_times) * 1e6,  # µs
            "success_rate": sum([1 for s in self.solve_status if s == cp.OPTIMAL])
            / len(self.solve_status)
            * 100,
        }

    def reset_statistics(self):
        """Reset solver statistics."""
        self.solve_times = []
        self.solve_status = []


def simulate_mpc_tracking(
    mpc: MPCController,
    trajectory_func,
    duration: float = 1.0,
    dt: float = 0.0001,
    x0: np.ndarray | None = None,
) -> Dict:
    """
    Simulate MPC tracking of a reference trajectory.

    Args:
        mpc: MPC controller instance
        trajectory_func: Function(t) -> (pos, vel, acc) that generates reference
        duration: Simulation duration (s)
        dt: Timestep (s)
        x0: Initial state [pos, vel, acc], defaults to [0, 0, 0]

    Returns:
        Dictionary with simulation results:
            - time: Time vector
            - x_actual: Actual state trajectory
            - x_ref: Reference trajectory
            - u: Control inputs (jerk)
            - tracking_error: Position tracking error
            - solve_times: MPC solve times
    """
    print(f"Simulating MPC tracking for {duration}s...")

    # Initialize
    n_steps = int(duration / dt)
    if x0 is None:
        x = np.array([0.0, 0.0, 0.0])
    else:
        x = x0.copy()

    # Storage
    time_vec = np.zeros(n_steps)
    x_actual = np.zeros((3, n_steps))
    x_ref_vec = np.zeros((3, n_steps))
    u_vec = np.zeros(n_steps)
    solve_times = np.zeros(n_steps)

    # MPC system model (for simulation)
    A_sim = mpc.A
    B_sim = mpc.B

    for i in range(n_steps):
        t = i * dt
        time_vec[i] = t

        # Get reference trajectory over prediction horizon
        x_ref_horizon = np.zeros((3, mpc.N + 1))
        for k in range(mpc.N + 1):
            t_future = t + k * dt
            pos_ref, vel_ref, acc_ref = trajectory_func(t_future)
            x_ref_horizon[:, k] = [pos_ref, vel_ref, acc_ref]

        # Store current reference
        x_ref_vec[:, i] = x_ref_horizon[:, 0]

        # Solve MPC
        u_opt, info = mpc.solve(x, x_ref_horizon)

        # Store
        x_actual[:, i] = x
        u_vec[i] = u_opt
        solve_times[i] = info["solve_time"]

        # Apply control to system (simulate dynamics)
        x = A_sim @ x + B_sim.flatten() * u_opt

        # Add small noise to make it realistic
        x += np.random.normal(0, 1e-6, 3)

    # Compute tracking error
    tracking_error = x_actual[0, :] - x_ref_vec[0, :]
    rms_error = np.sqrt(np.mean(tracking_error**2))

    print(f"  ✓ Simulation complete")
    print(f"  ✓ RMS tracking error: {np.rad2deg(rms_error):.3f}°")
    print(f"  ✓ Max tracking error: {np.rad2deg(np.max(np.abs(tracking_error))):.3f}°")
    print(f"  ✓ Mean solve time: {np.mean(solve_times)*1e6:.1f} µs")
    print(f"  ✓ Max solve time: {np.max(solve_times)*1e6:.1f} µs")

    return {
        "time": time_vec,
        "x_actual": x_actual,
        "x_ref": x_ref_vec,
        "u": u_vec,
        "tracking_error": tracking_error,
        "solve_times": solve_times,
        "rms_error": rms_error,
    }


def main():
    """Test MPC controller with S-curve trajectory."""
    print("=" * 80)
    print("MODEL PREDICTIVE CONTROL - PYTHON PROTOTYPE")
    print("=" * 80)

    # Load system model (from system identification)
    import json

    with open("motor_model.json", "r") as f:
        model_data = json.load(f)

    A = np.array(model_data["A_matrix"])
    B = np.array(model_data["B_matrix"])

    print("\nSystem model loaded:")
    print(f"  A matrix:\n{A}")
    print(f"  B matrix:\n{B.flatten()}")

    # Create MPC controller
    print("\nInitializing MPC controller...")
    mpc = MPCController(
        A=A,
        B=B,
        N=50,  # 50 steps = 5ms prediction horizon
        dt=0.0001,
        Q=np.array([100.0, 10.0, 1.0]),  # Position, velocity, accel weights
        R=0.01,  # Control effort weight
        v_max=2.0,
        a_max=5.0,
        j_max=100.0,
    )
    print("  ✓ MPC controller initialized")
    print(f"  ✓ Prediction horizon: {mpc.N} steps ({mpc.N * mpc.dt * 1e3:.1f} ms)")

    # Import S-curve generator (do it once)
    import sys
    from pathlib import Path
    sys.path.insert(0, str(Path(__file__).parent.parent))
    from demo_visualization import generate_scurve_trajectory

    # Define S-curve trajectory
    def scurve_trajectory(t):
        """Generate S-curve trajectory."""
        target = 1.57  # 90 degrees
        max_vel = 2.0
        max_accel = 5.0
        max_jerk = 100.0

        pos, vel, acc, jerk = generate_scurve_trajectory(t, target, max_vel, max_accel, max_jerk)
        return pos, vel, acc

    # Simulate MPC tracking
    print("\nRunning MPC simulation...")
    results = simulate_mpc_tracking(
        mpc=mpc, trajectory_func=scurve_trajectory, duration=0.6,  # 600ms (covers full trajectory)
        dt=0.0001, x0=np.array([0.0, 0.0, 0.0])
    )

    # Get MPC statistics
    stats = mpc.get_statistics()
    print(f"\n📊 MPC Solver Statistics:")
    print(f"  Total solves: {stats['count']}")
    print(f"  Success rate: {stats['success_rate']:.1f}%")
    print(f"  Mean solve time: {stats['mean_solve_time']:.1f} µs")
    print(f"  Max solve time: {stats['max_solve_time']:.1f} µs")
    print(f"  Std solve time: {stats['std_solve_time']:.1f} µs")

    # Compare to PID baseline
    print(f"\n🏆 Performance Comparison:")
    print(f"  PID (Phase 3):  RMS = 1.903°")
    print(f"  MPC (This run): RMS = {np.rad2deg(results['rms_error']):.3f}°")

    improvement = (1.903 - np.rad2deg(results["rms_error"])) / 1.903 * 100
    print(f"  Improvement:    {improvement:+.1f}%")

    if results["rms_error"] < np.deg2rad(1.0):
        print("\n🎯 SUCCESS: MPC achieved <1° RMS target!")
    else:
        print(f"\n⚠️  Close: Need {np.rad2deg(results['rms_error']) - 1.0:.2f}° more improvement")

    # Plot results
    try:
        import matplotlib.pyplot as plt

        fig, axes = plt.subplots(4, 1, figsize=(10, 10))

        t = results["time"]

        # Position tracking
        axes[0].plot(t, results["x_ref"][0, :], "k--", label="Reference", linewidth=2)
        axes[0].plot(t, results["x_actual"][0, :], "b-", label="MPC", linewidth=1.5)
        axes[0].set_ylabel("Position (rad)")
        axes[0].legend()
        axes[0].grid(True, alpha=0.3)
        axes[0].set_title("MPC Tracking Performance")

        # Tracking error
        axes[1].plot(t, np.rad2deg(results["tracking_error"]), "r-", linewidth=1)
        axes[1].axhline(0, color="k", linestyle="--", alpha=0.3)
        axes[1].set_ylabel("Error (°)")
        axes[1].grid(True, alpha=0.3)

        # Control input (jerk)
        axes[2].plot(t, results["u"], "g-", linewidth=1)
        axes[2].set_ylabel("Jerk (rad/s³)")
        axes[2].grid(True, alpha=0.3)

        # Solve time
        axes[3].plot(t, results["solve_times"] * 1e6, "m-", linewidth=1)
        axes[3].set_ylabel("Solve time (µs)")
        axes[3].set_xlabel("Time (s)")
        axes[3].grid(True, alpha=0.3)

        plt.tight_layout()
        plt.savefig("mpc_tracking_results.png", dpi=150)
        print("\n  ✓ Plot saved to: mpc_tracking_results.png")
    except ImportError:
        print("\n  ⚠️  matplotlib not available, skipping plot")

    print("\n" + "=" * 80)
    print("✅ MPC PROTOTYPE TEST COMPLETE")
    print("=" * 80)
    print("\nNext steps:")
    print("  1. Review mpc_tracking_results.png")
    print("  2. If performance is good, proceed to C implementation")
    print("  3. If not, tune Q/R weights or increase horizon N")


if __name__ == "__main__":
    main()
