#!/usr/bin/env python3
"""
Motor Physics Model - Universal Simulation

Provides realistic motor dynamics for testing and validation.
Used by:
  - demo_visualization.py
  - test_*.py scripts
  - Renode test validation

Physics:
  τ_motor = kt * i_q
  τ_net = τ_motor - τ_friction - τ_load
  α = τ_net / J
  ω = ∫ α dt
  θ = ∫ ω dt
"""

import numpy as np
from dataclasses import dataclass
from typing import Optional


@dataclass
class MotorParameters:
    """Physical parameters of the motor."""

    # Mechanical
    J: float = 0.001  # kg·m² - Rotor inertia
    b: float = 0.0005  # Nm·s/rad - Viscous damping

    # Electrical
    kt: float = 0.15  # Nm/A - Torque constant
    R: float = 1.0  # Ω - Phase resistance
    L: float = 0.001  # H - Phase inductance

    # Friction (Stribeck model)
    tau_coulomb: float = 0.02  # Nm - Coulomb friction
    tau_stribeck: float = 0.01  # Nm - Stribeck peak
    v_stribeck: float = 0.1  # rad/s - Stribeck velocity
    b_viscous: float = 0.001  # Nm·s/rad - Viscous friction

    # Temperature effects
    temp_nominal: float = 25.0  # °C
    temp_coeff: float = 0.005  # Friction temperature coefficient


class FrictionModel:
    """Stribeck friction model with temperature effects."""

    def __init__(self, params: MotorParameters):
        self.params = params

    def calculate(self, velocity: float, temperature: float = 25.0) -> float:
        """Calculate friction torque.

        Args:
            velocity: Angular velocity (rad/s)
            temperature: Motor temperature (°C)

        Returns:
            Friction torque (Nm)
        """
        p = self.params

        # Coulomb friction (sign of velocity)
        coulomb = p.tau_coulomb * np.sign(velocity) if abs(velocity) > 1e-6 else 0.0

        # Stribeck effect: friction peak at low speeds
        # τ_stribeck = τ_s * exp(-(v/v_s)²)
        stribeck = p.tau_stribeck * np.exp(-((velocity / p.v_stribeck) ** 2))
        stribeck *= np.sign(velocity) if abs(velocity) > 1e-6 else 0.0

        # Viscous friction (linear with velocity)
        viscous = p.b_viscous * velocity

        # Total friction
        tau_friction = coulomb + stribeck + viscous

        # Temperature effect (friction increases with temperature)
        temp_factor = 1.0 + p.temp_coeff * (temperature - p.temp_nominal)
        tau_friction *= temp_factor

        return tau_friction


class MotorDynamics:
    """Second-order motor dynamics with realistic physics.

    Dynamics:
        τ_motor = kt * i_q
        τ_net = τ_motor - b*ω - τ_friction - τ_load
        α = τ_net / J
        ω_new = ω + α * dt
        θ_new = θ + ω * dt

    This is the CORRECT physics, unlike kinematic approximations.
    """

    def __init__(self, params: Optional[MotorParameters] = None):
        """Initialize motor dynamics.

        Args:
            params: Motor parameters. If None, uses defaults.
        """
        self.params = params or MotorParameters()
        self.friction = FrictionModel(self.params)

        # State
        self.position = 0.0  # rad
        self.velocity = 0.0  # rad/s
        self.acceleration = 0.0  # rad/s²
        self.current_iq = 0.0  # A
        self.temperature = self.params.temp_nominal  # °C

    def update(
        self,
        i_q: float,
        external_load: float = 0.0,
        dt: float = 0.0001,
        temperature: Optional[float] = None,
    ) -> dict:
        """Update motor state by one timestep.

        Args:
            i_q: Quadrature current (A) - torque-producing current
            external_load: External load torque (Nm)
            dt: Time step (s)
            temperature: Motor temperature (°C). If None, uses internal state.

        Returns:
            Dictionary with motor state:
                - position (rad)
                - velocity (rad/s)
                - acceleration (rad/s²)
                - torque_motor (Nm)
                - torque_friction (Nm)
                - torque_net (Nm)
        """
        p = self.params

        # Use provided temperature or internal state
        temp = temperature if temperature is not None else self.temperature

        # Motor torque
        tau_motor = p.kt * i_q

        # Friction torque
        tau_friction = self.friction.calculate(self.velocity, temp)

        # Net torque
        tau_net = tau_motor - p.b * self.velocity - tau_friction - external_load

        # Dynamics (Newton's second law for rotation)
        self.acceleration = tau_net / p.J

        # Integrate (Euler method - simple and stable for small dt)
        self.velocity += self.acceleration * dt
        self.position += self.velocity * dt

        # Store current
        self.current_iq = i_q

        return {
            "position": self.position,
            "velocity": self.velocity,
            "acceleration": self.acceleration,
            "torque_motor": tau_motor,
            "torque_friction": tau_friction,
            "torque_net": tau_net,
            "current_iq": i_q,
        }

    def reset(self, position: float = 0.0, velocity: float = 0.0):
        """Reset motor state.

        Args:
            position: Initial position (rad)
            velocity: Initial velocity (rad/s)
        """
        self.position = position
        self.velocity = velocity
        self.acceleration = 0.0
        self.current_iq = 0.0


class FlexibleSystemDynamics:
    """Second-order flexible system for vibration analysis.

    Models a system with a flexible mode:
        G(s) = ω_n² / (s² + 2ζω_n·s + ω_n²)

    Used for input shaping validation.
    """

    def __init__(self, omega_n: float, zeta: float):
        """Initialize flexible system.

        Args:
            omega_n: Natural frequency (rad/s)
            zeta: Damping ratio (0-1)
        """
        self.omega_n = omega_n
        self.zeta = zeta

        # State
        self.position = 0.0
        self.velocity = 0.0

    def update(self, command: float, dt: float) -> float:
        """Update system state.

        Args:
            command: Input command
            dt: Time step (s)

        Returns:
            Current position
        """
        # Second-order dynamics
        # x'' + 2*zeta*omega_n*x' + omega_n^2*x = omega_n^2*u
        accel = (
            self.omega_n**2 * command
            - 2 * self.zeta * self.omega_n * self.velocity
            - self.omega_n**2 * self.position
        )

        # Euler integration
        self.velocity += accel * dt
        self.position += self.velocity * dt

        return self.position

    def reset(self):
        """Reset system state."""
        self.position = 0.0
        self.velocity = 0.0


class MotorSimulator:
    """High-level motor simulator combining dynamics and controller.

    This is a convenience wrapper for testing controllers.
    """

    def __init__(
        self,
        params: Optional[MotorParameters] = None,
        sample_rate: float = 10000.0,  # Hz
    ):
        """Initialize simulator.

        Args:
            params: Motor parameters
            sample_rate: Control loop frequency (Hz)
        """
        self.dynamics = MotorDynamics(params)
        self.dt = 1.0 / sample_rate

    def simulate_trajectory(
        self,
        controller_func,
        duration: float,
        external_load_func=None,
    ) -> dict:
        """Simulate motor following a trajectory.

        Args:
            controller_func: Function(t, state) -> i_q
                Takes time and state dict, returns i_q command
            duration: Simulation duration (s)
            external_load_func: Optional function(t) -> load_torque

        Returns:
            Dictionary with time history:
                - time: Time array
                - position: Position array
                - velocity: Velocity array
                - acceleration: Acceleration array
                - current_iq: Current array
                - torque_motor: Motor torque array
                - torque_friction: Friction torque array
        """
        n_samples = int(duration / self.dt)

        # Preallocate arrays
        time = np.zeros(n_samples)
        position = np.zeros(n_samples)
        velocity = np.zeros(n_samples)
        acceleration = np.zeros(n_samples)
        current_iq = np.zeros(n_samples)
        torque_motor = np.zeros(n_samples)
        torque_friction = np.zeros(n_samples)

        for i in range(n_samples):
            t = i * self.dt
            time[i] = t

            # Get current state
            state = {
                "time": t,
                "position": self.dynamics.position,
                "velocity": self.dynamics.velocity,
                "acceleration": self.dynamics.acceleration,
            }

            # Controller computes i_q
            i_q = controller_func(t, state)

            # External load (if any)
            load = external_load_func(t) if external_load_func else 0.0

            # Update dynamics
            result = self.dynamics.update(i_q, external_load=load, dt=self.dt)

            # Store
            position[i] = result["position"]
            velocity[i] = result["velocity"]
            acceleration[i] = result["acceleration"]
            current_iq[i] = result["current_iq"]
            torque_motor[i] = result["torque_motor"]
            torque_friction[i] = result["torque_friction"]

        return {
            "time": time,
            "position": position,
            "velocity": velocity,
            "acceleration": acceleration,
            "current_iq": current_iq,
            "torque_motor": torque_motor,
            "torque_friction": torque_friction,
        }


# Convenience function for quick tests
def quick_step_test(
    i_q_step: float = 1.0,
    duration: float = 0.5,
    params: Optional[MotorParameters] = None,
) -> dict:
    """Run a quick step response test.

    Args:
        i_q_step: Step current (A)
        duration: Test duration (s)
        params: Motor parameters

    Returns:
        Dictionary with time history
    """
    sim = MotorSimulator(params)

    def controller(t, state):
        return i_q_step

    return sim.simulate_trajectory(controller, duration)


if __name__ == "__main__":
    # Demo: step response
    print("=" * 60)
    print("Motor Physics Model - Step Response Demo")
    print("=" * 60)

    result = quick_step_test(i_q_step=2.0, duration=0.5)

    print(f"\nFinal state:")
    print(f"  Position: {result['position'][-1]:.3f} rad")
    print(f"  Velocity: {result['velocity'][-1]:.3f} rad/s")
    print(f"  Max torque: {np.max(result['torque_motor']):.3f} Nm")

    print("\n✅ Motor model ready for use!")

