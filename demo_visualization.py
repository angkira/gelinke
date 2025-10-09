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


# Hardware configuration (matches firmware)
class HardwareConfig:
    """Hardware limits and thermal protection settings."""

    # Current limits
    MAX_CONTINUOUS_CURRENT = 5.0    # A - Safe continuous operation
    MAX_PEAK_CURRENT = 10.0         # A - Short-term peak (< 1s)
    THERMAL_SHUTDOWN_CURRENT = 12.0 # A - Emergency shutdown

    # Temperature limits
    TEMP_NOMINAL = 25.0             # °C - Ambient temperature
    TEMP_WARNING = 60.0             # °C - Start derating
    TEMP_CRITICAL = 80.0            # °C - Heavy derating
    TEMP_SHUTDOWN = 90.0            # °C - Emergency shutdown

    # Thermal derating (reduces current limit based on temperature)
    DERATING_START_TEMP = 60.0      # °C - Start reducing current
    DERATING_FULL_TEMP = 80.0       # °C - Maximum derating
    DERATING_MIN_FACTOR = 0.5       # Minimum 50% current at hot temps

    # Motor parameters (from telemetry.rs)
    KT = 0.15                       # Nm/A - Torque constant
    R_PHASE = 1.0                   # Ω - Phase resistance
    THERMAL_MASS = 100.0            # J/K - Motor thermal mass
    COOLING_RATE = 0.5              # W/K - Heat dissipation rate


def apply_current_limit(i_q: float, temperature: float, config: HardwareConfig = HardwareConfig()) -> tuple[float, bool]:
    """Apply current limit with temperature derating.

    Args:
        i_q: Requested Q-axis current (A)
        temperature: Motor temperature (°C)
        config: Hardware configuration

    Returns:
        Tuple of (limited_current, is_saturated)
    """
    # Temperature-based derating
    if temperature >= config.TEMP_SHUTDOWN:
        # Emergency shutdown
        return 0.0, True

    if temperature >= config.DERATING_START_TEMP:
        # Linear derating between DERATING_START_TEMP and DERATING_FULL_TEMP
        derating_range = config.DERATING_FULL_TEMP - config.DERATING_START_TEMP
        temp_above_start = temperature - config.DERATING_START_TEMP
        derating_factor = 1.0 - (1.0 - config.DERATING_MIN_FACTOR) * (temp_above_start / derating_range)
        derating_factor = max(config.DERATING_MIN_FACTOR, derating_factor)
    else:
        derating_factor = 1.0

    # Apply derating to peak current limit
    effective_limit = config.MAX_PEAK_CURRENT * derating_factor

    # For continuous operation, use lower limit
    continuous_limit = config.MAX_CONTINUOUS_CURRENT * derating_factor

    # Clamp current
    i_q_limited = np.clip(i_q, -effective_limit, effective_limit)
    is_saturated = abs(i_q) > effective_limit

    return i_q_limited, is_saturated


def simulate_temperature(
    i_q: float,
    temperature: float,
    dt: float,
    config: HardwareConfig = HardwareConfig()
) -> float:
    """Simulate motor temperature based on current and cooling.

    Simple thermal model: dT/dt = (P_loss - P_cooling) / C_thermal
    where P_loss = I²R and P_cooling = k * (T - T_ambient)

    Args:
        i_q: Q-axis current (A)
        temperature: Current temperature (°C)
        dt: Time step (s)
        config: Hardware configuration

    Returns:
        New temperature (°C)
    """
    # Power loss (I²R heating)
    power_loss = i_q**2 * config.R_PHASE

    # Cooling power (proportional to temperature difference)
    temp_diff = temperature - config.TEMP_NOMINAL
    power_cooling = config.COOLING_RATE * temp_diff

    # Net power
    power_net = power_loss - power_cooling

    # Temperature change: dT = P * dt / C
    dT = power_net * dt / config.THERMAL_MASS

    # Update temperature
    new_temp = temperature + dT

    # Clamp to realistic range
    return np.clip(new_temp, config.TEMP_NOMINAL, config.TEMP_SHUTDOWN + 10.0)


# ============================================================================
# Advanced Load Estimation: Disturbance Observer
# ============================================================================

class FrictionModel:
    """Advanced friction model with Coulomb, viscous, and Stribeck effects."""

    def __init__(
        self,
        tau_coulomb: float = 0.02,      # Nm - Static/Coulomb friction
        b_viscous: float = 0.001,       # Nm·s/rad - Viscous damping
        v_stribeck: float = 0.1,        # rad/s - Stribeck velocity
        tau_stribeck: float = 0.01,     # Nm - Stribeck peak torque
        temp_coeff: float = 0.005,      # Temperature coefficient (per °C)
    ):
        self.tau_coulomb = tau_coulomb
        self.b_viscous = b_viscous
        self.v_stribeck = v_stribeck
        self.tau_stribeck = tau_stribeck
        self.temp_coeff = temp_coeff

    def calculate(self, velocity: float, temperature: float = 25.0) -> float:
        """Calculate friction torque.

        Args:
            velocity: Angular velocity (rad/s)
            temperature: Motor temperature (°C)

        Returns:
            Friction torque (Nm)
        """
        # Temperature factor (friction increases with temperature)
        temp_factor = 1.0 + self.temp_coeff * (temperature - 25.0)

        # Stribeck effect: friction peak at low speeds
        # τ_stribeck = τ_s * exp(-(v/v_s)²)
        stribeck = self.tau_stribeck * np.exp(-(velocity / self.v_stribeck)**2)

        # Viscous friction (linear with velocity)
        tau_viscous = self.b_viscous * velocity * temp_factor

        # Coulomb friction (constant, direction-dependent)
        if abs(velocity) < 0.001:
            # At very low speeds, friction is not well-defined (stiction)
            sign = 0.0
        else:
            sign = np.sign(velocity)

        tau_coulomb = sign * self.tau_coulomb * temp_factor

        # Total friction
        tau_friction = tau_coulomb + sign * stribeck + tau_viscous

        return tau_friction


class DisturbanceObserver:
    """Momentum-based disturbance observer for load estimation.

    Estimates external load using physics-based approach:
        τ_load = τ_motor - J·α - b·ω - τ_friction

    Advantages over baseline subtraction:
    - Works during motion (not just steady state)
    - Better noise rejection
    - Can separate friction from external load
    - Physics-based, more robust
    """

    def __init__(
        self,
        J: float = 0.001,           # kg·m² - Rotor inertia
        b: float = 0.0005,          # Nm·s/rad - Viscous damping
        kt: float = 0.15,           # Nm/A - Torque constant
        alpha: float = 0.05,        # Filter coefficient (0-1)
        friction_model: FrictionModel | None = None,
        compensate_friction: bool = True,
    ):
        self.J = J
        self.b = b
        self.kt = kt
        self.alpha = alpha
        self.friction_model = friction_model or FrictionModel()
        self.compensate_friction = compensate_friction

        # State
        self.load_estimate = 0.0
        self.prev_velocity = 0.0
        self.initialized = False

        # Diagnostics
        self.tau_motor_history = []
        self.tau_motion_history = []
        self.tau_friction_history = []

    def update(
        self,
        velocity: float,
        i_q: float,
        dt: float,
        temperature: float = 25.0
    ) -> float:
        """Update observer with new measurements.

        Args:
            velocity: Angular velocity (rad/s)
            i_q: Q-axis current (A)
            dt: Time step (s)
            temperature: Motor temperature (°C)

        Returns:
            Estimated external load torque (Nm)
        """
        # Calculate acceleration (numerical derivative)
        if not self.initialized:
            accel = 0.0
            self.prev_velocity = velocity
            self.initialized = True
        else:
            if dt > 0:
                accel = (velocity - self.prev_velocity) / dt
            else:
                accel = 0.0
            self.prev_velocity = velocity

        # Motor torque (from current measurement)
        tau_motor = self.kt * i_q

        # Expected torque for rigid-body motion
        # τ_motion = J·α + b·ω
        tau_motion = self.J * accel + self.b * velocity

        # Friction torque (model-based)
        tau_friction = self.friction_model.calculate(velocity, temperature)

        # Disturbance torque (includes external load + model error)
        # τ_disturbance = τ_motor - τ_motion - τ_friction
        if self.compensate_friction:
            tau_disturbance = tau_motor - tau_motion - tau_friction
        else:
            # Don't compensate friction (estimate it as part of load)
            tau_disturbance = tau_motor - tau_motion

        # Low-pass filter for noise rejection
        # load_estimate[k] = α·τ_dist[k] + (1-α)·load_estimate[k-1]
        self.load_estimate = (
            self.alpha * tau_disturbance +
            (1 - self.alpha) * self.load_estimate
        )

        # Store for diagnostics
        self.tau_motor_history.append(tau_motor)
        self.tau_motion_history.append(tau_motion)
        self.tau_friction_history.append(tau_friction)

        return self.load_estimate

    def reset(self):
        """Reset observer state."""
        self.load_estimate = 0.0
        self.prev_velocity = 0.0
        self.initialized = False
        self.tau_motor_history.clear()
        self.tau_motion_history.clear()
        self.tau_friction_history.clear()

    def get_diagnostics(self) -> dict:
        """Get diagnostic information for analysis."""
        return {
            'tau_motor': np.array(self.tau_motor_history) if self.tau_motor_history else np.array([]),
            'tau_motion': np.array(self.tau_motion_history) if self.tau_motion_history else np.array([]),
            'tau_friction': np.array(self.tau_friction_history) if self.tau_friction_history else np.array([]),
        }


def calculate_control_metrics(
    position: np.ndarray,
    target_position: float,
    velocity: np.ndarray,
    max_vel: float,
    dt: float,
) -> dict:
    """Calculate control quality metrics from simulation results.

    Args:
        position: Position array
        target_position: Target position (rad)
        velocity: Velocity array
        max_vel: Maximum velocity limit (rad/s)
        dt: Time step (s)

    Returns:
        Dictionary with control metrics
    """
    # Find when position first crosses target
    cross_idx = np.where(position >= target_position)[0]

    if len(cross_idx) == 0:
        # Never reached target
        return {
            "overshoot_percent": 0.0,
            "overshoot_rad": 0.0,
            "max_position": np.max(position),
            "final_position": position[-1],
            "settling_time": None,
            "rms_error_deg": np.rad2deg(
                np.sqrt(np.mean((position - target_position) ** 2))
            ),
            "max_error_deg": np.rad2deg(np.max(np.abs(position - target_position))),
            "max_velocity": np.max(np.abs(velocity)),
            "velocity_violation_percent": max(
                0.0, (np.max(np.abs(velocity)) - max_vel) / max_vel * 100
            ),
            "reached_target": False,
        }

    first_cross = cross_idx[0]

    # Overshoot
    max_pos = np.max(position[first_cross:])
    overshoot_rad = max_pos - target_position
    overshoot_percent = (
        (overshoot_rad / target_position * 100) if target_position > 0 else 0.0
    )

    # Settling time (within 2% of target)
    tolerance = 0.02 * target_position
    settled = np.abs(position[first_cross:] - target_position) < tolerance

    # Find first index where it settles and stays settled (at least 100 samples)
    settling_time = None
    for idx in range(len(settled) - 100):
        if np.all(settled[idx : idx + 100]):
            settling_time = (first_cross + idx) * dt
            break

    # Tracking error
    error = position - target_position
    rms_error_rad = np.sqrt(np.mean(error**2))
    max_error = np.max(np.abs(error))

    # Velocity violations
    max_velocity = np.max(np.abs(velocity))
    velocity_violation = max(0.0, max_velocity - max_vel)
    velocity_violation_percent = (
        (velocity_violation / max_vel * 100) if max_vel > 0 else 0.0
    )

    return {
        "overshoot_percent": overshoot_percent,
        "overshoot_rad": overshoot_rad,
        "max_position": max_pos,
        "final_position": position[-1],
        "settling_time": settling_time,
        "rms_error_deg": np.rad2deg(rms_error_rad),
        "max_error_deg": np.rad2deg(max_error),
        "max_velocity": max_velocity,
        "velocity_violation_percent": velocity_violation_percent,
        "reached_target": True,
    }


class PIDController:
    """PID controller with anti-windup and saturation."""

    def __init__(
        self,
        kp: float,
        ki: float,
        kd: float,
        max_integral: float = 10.0,
        max_output: float | None = None,
        output_offset: float = 0.0,
    ):
        self.kp = kp
        self.ki = ki
        self.kd = kd
        self.max_integral = max_integral
        self.max_output = max_output
        self.output_offset = output_offset

        self.integral = 0.0
        self.prev_error = 0.0
        self.initialized = False

    def update(self, error: float, dt: float, feedforward: float = 0.0) -> float:
        """Update PID controller with error and return control output.

        Args:
            error: Current error (target - actual)
            dt: Time step
            feedforward: Feedforward term to add to output

        Returns:
            Control output
        """
        # Proportional term
        p_term = self.kp * error

        # Integral term with anti-windup
        self.integral += error * dt
        self.integral = np.clip(self.integral, -self.max_integral, self.max_integral)
        i_term = self.ki * self.integral

        # Derivative term (with initialization protection)
        if not self.initialized:
            d_term = 0.0
            self.prev_error = error
            self.initialized = True
        else:
            d_term = self.kd * (error - self.prev_error) / dt if dt > 0 else 0.0
            self.prev_error = error

        # Compute output
        output = p_term + i_term + d_term + feedforward + self.output_offset

        # Apply saturation
        if self.max_output is not None:
            output = np.clip(output, -self.max_output, self.max_output)

        return output

    def reset(self):
        """Reset controller state."""
        self.integral = 0.0
        self.prev_error = 0.0
        self.initialized = False


def generate_scurve_trajectory(
    t: float,
    target_pos: float,
    max_vel: float,
    max_accel: float,
    max_jerk: float,
) -> tuple[float, float, float, float]:
    """Generate S-curve (jerk-limited) trajectory at time t.

    S-curve trajectory eliminates acceleration discontinuities by limiting jerk,
    resulting in smoother motion that's easier for the controller to track.

    Args:
        t: Current time (s)
        target_pos: Target position (rad)
        max_vel: Maximum velocity (rad/s)
        max_accel: Maximum acceleration (rad/s²)
        max_jerk: Maximum jerk (rad/s³)

    Returns:
        Tuple of (position, velocity, acceleration, jerk)
    """
    # Phase durations
    t_jerk = max_accel / max_jerk  # Time to reach max acceleration
    t_accel_const = max_vel / max_accel - t_jerk  # Constant accel time

    # Handle case where we don't reach max velocity
    if t_accel_const < 0:
        # Triangular velocity profile
        t_jerk = np.sqrt(max_vel / max_jerk)
        t_accel_const = 0.0
        effective_accel = max_jerk * t_jerk
    else:
        effective_accel = max_accel

    # Acceleration phase distance
    x_jerk1 = (1/6) * max_jerk * t_jerk**3
    x_accel_const = effective_accel * t_jerk * t_accel_const + 0.5 * effective_accel * t_accel_const**2
    x_jerk2 = effective_accel * t_jerk * t_jerk - (1/6) * max_jerk * t_jerk**3
    x_accel = x_jerk1 + x_accel_const + x_jerk2

    v_max = effective_accel * t_jerk + effective_accel * t_accel_const

    # Coast phase
    if 2 * x_accel < target_pos:
        x_coast = target_pos - 2 * x_accel
        t_coast = x_coast / v_max
    else:
        # No coast, reduce velocity
        t_coast = 0.0
        # Recalculate for shorter distance
        scale = np.sqrt(target_pos / (2 * x_accel))
        t_jerk = t_jerk * scale
        t_accel_const = t_accel_const * scale
        effective_accel = effective_accel * scale
        v_max = v_max * scale
        x_accel = target_pos / 2
        x_coast = 0

    # Phase boundaries
    t1 = t_jerk  # End of jerk-up
    t2 = t1 + t_accel_const  # End of constant accel
    t3 = t2 + t_jerk  # End of jerk-down (peak velocity)
    t4 = t3 + t_coast  # End of coast
    t5 = t4 + t_jerk  # End of decel jerk-down
    t6 = t5 + t_accel_const  # End of constant decel
    t7 = t6 + t_jerk  # End of decel jerk-up (stopped)

    # Calculate trajectory at time t
    if t < t1:
        # Phase 1: Jerk up (increasing acceleration)
        jerk = max_jerk
        accel = max_jerk * t
        vel = 0.5 * max_jerk * t**2
        pos = (1/6) * max_jerk * t**3
    elif t < t2:
        # Phase 2: Constant acceleration
        t_rel = t - t1
        jerk = 0.0
        accel = effective_accel
        vel = effective_accel * t_jerk + effective_accel * t_rel
        pos = x_jerk1 + effective_accel * t_jerk * t_rel + 0.5 * effective_accel * t_rel**2
    elif t < t3:
        # Phase 3: Jerk down (decreasing acceleration)
        t_rel = t - t2
        jerk = -max_jerk
        accel = effective_accel - max_jerk * t_rel
        vel = effective_accel * (t_jerk + t_accel_const) + effective_accel * t_rel - 0.5 * max_jerk * t_rel**2
        pos = x_jerk1 + x_accel_const + effective_accel * t_jerk * t_rel + effective_accel * t_rel**2 / 2 - (1/6) * max_jerk * t_rel**3
    elif t < t4:
        # Phase 4: Coast (constant velocity)
        t_rel = t - t3
        jerk = 0.0
        accel = 0.0
        vel = v_max
        pos = x_accel + v_max * t_rel
    elif t < t5:
        # Phase 5: Decel jerk down (increasing negative acceleration)
        t_rel = t - t4
        jerk = -max_jerk
        accel = -max_jerk * t_rel
        vel = v_max - 0.5 * max_jerk * t_rel**2
        pos = x_accel + x_coast + v_max * t_rel - (1/6) * max_jerk * t_rel**3
    elif t < t6:
        # Phase 6: Constant deceleration
        t_rel = t - t5
        jerk = 0.0
        accel = -effective_accel
        vel = v_max - effective_accel * t_jerk - effective_accel * t_rel
        pos = x_accel + x_coast + (v_max - effective_accel * t_jerk / 2) * t_jerk + (v_max - effective_accel * t_jerk) * t_rel - 0.5 * effective_accel * t_rel**2
    elif t < t7:
        # Phase 7: Decel jerk up (decreasing negative acceleration)
        t_rel = t - t6
        jerk = max_jerk
        accel = -effective_accel + max_jerk * t_rel
        vel_at_t6 = v_max - effective_accel * (t_jerk + t_accel_const)
        vel = vel_at_t6 - effective_accel * t_rel + 0.5 * max_jerk * t_rel**2
        pos_at_t6 = target_pos - x_jerk2
        pos = pos_at_t6 + vel_at_t6 * t_rel - 0.5 * effective_accel * t_rel**2 + (1/6) * max_jerk * t_rel**3
    else:
        # Phase 8: Settled at target
        jerk = 0.0
        accel = 0.0
        vel = 0.0
        pos = target_pos

    return pos, vel, accel, jerk


def simulate_trapezoidal_motion(
    use_improved_controller: bool = True,
    kp_pos: float = 8.0,
    kp_vel: float = 3.0,
    ki_vel: float = 0.5,
    kd_vel: float = 0.1,
    kff_vel: float = 1.0,
    kff_accel: float = 0.0,
    trajectory_type: str = "trapezoidal",
    max_jerk: float = 50.0,
):
    """Simulate motion profile with FOC control.

    Args:
        use_improved_controller: If True, use improved PID cascade controller
        kp_pos: Position proportional gain
        kp_vel: Velocity proportional gain
        ki_vel: Velocity integral gain
        kd_vel: Velocity derivative gain
        kff_vel: Velocity feedforward gain (0-1, typically 1.0)
        kff_accel: Acceleration feedforward gain (typically 0.1-0.3)
        trajectory_type: "trapezoidal" or "scurve"
        max_jerk: Maximum jerk for S-curve trajectory (rad/s³)
    """
    controller_type = "improved" if use_improved_controller else "original"
    print(f"\n📊 Simulating {trajectory_type.upper()} Motion Profile ({controller_type})...")

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

    # Initialize improved controller (cascade: position -> velocity)
    if use_improved_controller:
        vel_controller = PIDController(
            kp=kp_vel,
            ki=ki_vel,
            kd=kd_vel,
            max_integral=max_vel,  # Limit integral windup to max velocity
            max_output=max_accel,  # Acceleration saturation
        )

    for i in range(n_samples):
        t = i * dt

        # Motion profile with acceleration tracking
        if trajectory_type == "scurve":
            # S-curve trajectory (jerk-limited)
            target_pos, target_vel, target_accel, target_jerk = generate_scurve_trajectory(
                t, target, max_vel, max_accel, max_jerk
            )
        else:
            # Trapezoidal trajectory (original)
            if t < t_accel:
                # Acceleration phase
                target_vel = max_accel * t
                target_pos = 0.5 * max_accel * t**2
                target_accel = max_accel
            elif t < t_accel + t_coast:
                # Coast phase
                target_vel = max_vel
                target_pos = 0.5 * max_accel * t_accel**2 + max_vel * (t - t_accel)
                target_accel = 0.0
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
                target_accel = -max_accel
            else:
                # Settling
                target_vel = 0.0
                target_pos = target
                target_accel = 0.0

        # FOC Controller
        pos_error = target_pos - position

        if use_improved_controller:
            # Improved cascade control architecture with feedforward
            # Outer loop (position): P controller -> target velocity
            target_vel_from_pos = kp_pos * pos_error
            target_vel_from_pos = np.clip(target_vel_from_pos, -max_vel, max_vel)

            # Combine with feedforward velocity from trajectory
            target_vel_combined = kff_vel * target_vel + target_vel_from_pos

            # Inner loop (velocity): PID controller -> acceleration (feedback)
            vel_error = target_vel_combined - velocity
            accel_fb = vel_controller.update(vel_error, dt, feedforward=0.0)

            # Add acceleration feedforward (reduces lag)
            accel_ff = kff_accel * target_accel
            accel = accel_fb + accel_ff

            # Apply acceleration with saturation
            accel = np.clip(accel, -max_accel, max_accel)
            velocity += accel * dt
            velocity = np.clip(velocity, -max_vel, max_vel)
            position += velocity * dt

        else:
            # Original (broken) controller for comparison
            vel_error = target_vel - velocity

            # Original PI gains (with issues)
            kp_pos_orig = 20.0
            kp_vel_orig = 0.5
            # ki_vel = 2.0  # Declared but never used in original!

            # Original control law (incorrect)
            velocity += (kp_pos_orig * pos_error + kp_vel_orig * vel_error) * dt
            position += velocity * dt

            accel = kp_pos_orig * pos_error + kp_vel_orig * vel_error

        # Current (I_q) proportional to acceleration + friction
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
    """Simulate adaptive control response to load disturbance with improved load estimation."""
    print("\n📊 Simulating Adaptive Control Load Step...")

    collector = TestDataCollector("demo_adaptive_load_step")

    dt = 0.0001
    duration = 0.6  # 600 ms
    n_samples = int(duration / dt)

    position = 0.0
    velocity = 0.0
    target_pos = 1.0  # Hold position at 1.0 rad

    # Load estimation state with baseline learning
    load_estimate = 0.0
    i_q_baseline = 0.0
    baseline_learned = False
    baseline_samples = []

    # coolStep state
    coolstep_enabled = True
    current_reduction_factor = 1.0

    for i in range(n_samples):
        t = i * dt

        # Apply external load disturbance at t=0.2s
        if 0.2 <= t < 0.4:
            external_load = 0.3  # 0.3 Nm disturbance
        else:
            external_load = 0.0

        # IMPROVED Position controller with adaptive gain and deadband
        pos_error = target_pos - position

        # Adaptive gain: lower for small errors to reduce holding current
        if abs(pos_error) < 0.01:  # Within 0.01 rad (0.57°)
            kp = 2.0  # Very low for tiny errors
        else:
            kp = 8.0  # Normal gain for larger errors (from Phase 1 tuning)

        kd = 3.0  # Higher damping for stability

        accel = kp * pos_error - kd * velocity
        velocity += accel * dt
        position += velocity * dt

        # Current (with external load)
        i_q_base = 0.1 * accel + 0.05 * velocity + external_load / 0.15

        # IMPROVED Load estimation with baseline learning
        if not baseline_learned:
            # Learn baseline during first 0.15s (before load step at 0.2s)
            if t < 0.15:
                baseline_samples.append(i_q_base)
            else:
                # Calculate baseline from samples
                if baseline_samples:
                    i_q_baseline = np.mean(baseline_samples)
                    print(f"   ✓ Learned baseline current: {i_q_baseline:.3f} A "
                          f"(torque: {0.15 * i_q_baseline:.3f} Nm)")
                else:
                    i_q_baseline = 0.0
                baseline_learned = True

        # External load estimation = current - baseline
        if baseline_learned:
            i_q_external = i_q_base - i_q_baseline
            load_estimate_raw = 0.15 * i_q_external  # Convert to torque

            # Low-pass filter with faster response
            alpha = 0.05  # 5% (was 1% - now responds faster)
            load_estimate = alpha * load_estimate_raw + (1 - alpha) * load_estimate
        else:
            # During learning phase, estimate is zero
            load_estimate = 0.0

        # coolStep: Reduce current when load is steady
        if coolstep_enabled and baseline_learned:
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
    """Simulate high-speed motion with saturation and thermal protection."""
    print("\n📊 Simulating High-Speed Motion with Thermal Protection...")

    collector = TestDataCollector("demo_high_speed_motion")

    dt = 0.0001
    duration = 1.0
    n_samples = int(duration / dt)

    position = 0.0
    velocity = 0.0
    target = 6.28  # 360 degrees
    max_vel = 10.0  # Very fast
    max_accel = 50.0

    # Hardware config and thermal state
    hw_config = HardwareConfig()
    temperature = hw_config.TEMP_NOMINAL
    saturation_count = 0

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

        # Calculate requested current
        i_q_requested = 0.2 * accel + 0.1 * velocity

        # Apply hardware current limits with thermal protection
        i_q, is_saturated = apply_current_limit(i_q_requested, temperature, hw_config)
        i_d = 0.0

        if is_saturated:
            saturation_count += 1

        # Simulate temperature based on actual current
        temperature = simulate_temperature(i_q, temperature, dt, hw_config)

        # Load
        load = hw_config.KT * i_q

        # PWM (with saturation)
        theta = position
        duty_a = 0.5 + 0.4 * i_q * np.cos(theta)
        duty_b = 0.5 + 0.4 * i_q * np.cos(theta - 2 * np.pi / 3)
        duty_c = 0.5 + 0.4 * i_q * np.cos(theta + 2 * np.pi / 3)

        # Hard saturation
        duty_a = np.clip(duty_a, 0.0, 1.0)
        duty_b = np.clip(duty_b, 0.0, 1.0)
        duty_c = np.clip(duty_c, 0.0, 1.0)

        # Health degrades with temperature and saturation
        temp_factor = (temperature - hw_config.TEMP_NOMINAL) / (hw_config.TEMP_SHUTDOWN - hw_config.TEMP_NOMINAL)
        saturation_factor = saturation_count / (i + 1) if i > 0 else 0
        health = 100.0 - 15.0 * temp_factor - 10.0 * saturation_factor
        health = max(health, 60.0)

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
                temperature=temperature,
                health_score=health,
            )

    # Save
    output_dir = Path("demo_results")
    collector.save_json(str(output_dir / "demo_high_speed_motion.json"))
    collector.save_pandas_csv(str(output_dir / "demo_high_speed_motion.csv"))

    # Report thermal protection statistics
    saturation_percent = saturation_count / n_samples * 100
    print(f"   ✓ Generated {len(collector.snapshots)} samples")
    print(f"   ✓ Max velocity: {max_vel} rad/s")
    print(f"   ✓ Final temperature: {temperature:.1f}°C (started at {hw_config.TEMP_NOMINAL}°C)")
    print(f"   ✓ Current saturation: {saturation_percent:.1f}% of time")
    if temperature >= hw_config.TEMP_WARNING:
        print(f"   ⚠️  Temperature reached warning level ({hw_config.TEMP_WARNING}°C)")

    return str(output_dir / "demo_high_speed_motion.json")


def tune_controller_gains():
    """Systematically tune controller gains and compare metrics."""
    print("\n" + "=" * 80)
    print("🔧 CONTROLLER GAIN TUNING")
    print("=" * 80)

    # Test configurations based on theoretical analysis
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
            "name": "Option 1: Reduced kp_pos, increased kp_vel",
            "use_improved": True,
            "kp_pos": 10.0,
            "kp_vel": 2.0,
            "ki_vel": 1.0,
            "kd_vel": 0.0,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 2: Conservative damping",
            "use_improved": True,
            "kp_pos": 8.0,
            "kp_vel": 3.0,
            "ki_vel": 0.5,
            "kd_vel": 0.0,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 3: Critically damped",
            "use_improved": True,
            "kp_pos": 5.0,
            "kp_vel": 2.5,
            "ki_vel": 0.3,
            "kd_vel": 0.0,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 4: With D-term (low)",
            "use_improved": True,
            "kp_pos": 8.0,
            "kp_vel": 3.0,
            "ki_vel": 0.5,
            "kd_vel": 0.1,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 5: With D-term (medium)",
            "use_improved": True,
            "kp_pos": 8.0,
            "kp_vel": 3.0,
            "ki_vel": 0.5,
            "kd_vel": 0.2,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 6: Aggressive with D-term",
            "use_improved": True,
            "kp_pos": 10.0,
            "kp_vel": 4.0,
            "ki_vel": 0.8,
            "kd_vel": 0.15,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 7: High D-term for damping",
            "use_improved": True,
            "kp_pos": 8.0,
            "kp_vel": 3.5,
            "ki_vel": 0.5,
            "kd_vel": 0.5,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 8: Very high D-term",
            "use_improved": True,
            "kp_pos": 6.0,
            "kp_vel": 4.0,
            "ki_vel": 0.4,
            "kd_vel": 0.8,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 9: Lower kp_pos, high damping",
            "use_improved": True,
            "kp_pos": 4.0,
            "kp_vel": 3.0,
            "ki_vel": 0.3,
            "kd_vel": 0.6,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 10: Balanced with strong D",
            "use_improved": True,
            "kp_pos": 6.0,
            "kp_vel": 3.5,
            "ki_vel": 0.4,
            "kd_vel": 1.0,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        # Advanced options with feedforward
        {
            "name": "Option 11: Higher I-gain (fix steady-state)",
            "use_improved": True,
            "kp_pos": 6.0,
            "kp_vel": 3.5,
            "ki_vel": 1.0,  # Increased from 0.4
            "kd_vel": 1.0,
            "kff_vel": 1.0,
            "kff_accel": 0.0,
        },
        {
            "name": "Option 12: With accel feedforward",
            "use_improved": True,
            "kp_pos": 6.0,
            "kp_vel": 3.5,
            "ki_vel": 0.4,
            "kd_vel": 1.0,
            "kff_vel": 1.0,
            "kff_accel": 0.2,  # Added acceleration feedforward
        },
        {
            "name": "Option 13: Higher I + accel FF",
            "use_improved": True,
            "kp_pos": 6.0,
            "kp_vel": 3.5,
            "ki_vel": 1.0,
            "kd_vel": 1.0,
            "kff_vel": 1.0,
            "kff_accel": 0.2,
        },
        {
            "name": "Option 14: Aggressive I + strong FF",
            "use_improved": True,
            "kp_pos": 6.0,
            "kp_vel": 3.5,
            "ki_vel": 1.5,
            "kd_vel": 1.0,
            "kff_vel": 1.0,
            "kff_accel": 0.3,
        },
        {
            "name": "Option 15: Conservative with FF",
            "use_improved": True,
            "kp_pos": 5.0,
            "kp_vel": 3.0,
            "ki_vel": 1.2,
            "kd_vel": 0.8,
            "kff_vel": 1.0,
            "kff_accel": 0.25,
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

        # Run simulation with this config (without saving)
        # We need to extract just the simulation part
        target = 1.57
        max_vel = 2.0
        max_accel = 5.0
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

        position_arr = []
        velocity_arr = []

        position = 0.0
        velocity = 0.0

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

            # Motion profile with acceleration
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
            pos_error = target_pos - position

            if config["use_improved"]:
                target_vel_from_pos = config["kp_pos"] * pos_error
                target_vel_from_pos = np.clip(target_vel_from_pos, -max_vel, max_vel)
                target_vel_combined = kff_v * target_vel + target_vel_from_pos

                vel_error = target_vel_combined - velocity
                accel_fb = vel_controller.update(vel_error, dt, feedforward=0.0)
                accel_ff = kff_a * target_accel
                accel = accel_fb + accel_ff
                accel = np.clip(accel, -max_accel, max_accel)

                velocity += accel * dt
                velocity = np.clip(velocity, -max_vel, max_vel)
                position += velocity * dt
            else:
                vel_error = target_vel - velocity
                velocity += (
                    config["kp_pos"] * pos_error + config["kp_vel"] * vel_error
                ) * dt
                position += velocity * dt

            position_arr.append(position)
            velocity_arr.append(velocity)

        # Calculate metrics
        position_arr = np.array(position_arr)
        velocity_arr = np.array(velocity_arr)
        metrics = calculate_control_metrics(position_arr, target, velocity_arr, 2.0, dt)

        # Calculate theoretical damping ratio
        omega_n = np.sqrt(config["kp_pos"])
        zeta = config["kp_vel"] / (2 * omega_n) if omega_n > 0 else 0

        metrics["config"] = config
        metrics["omega_n"] = omega_n
        metrics["zeta"] = zeta

        results.append(metrics)

        # Print results
        print(f"   ωn={omega_n:.2f} rad/s, ζ={zeta:.3f}")
        print(f"   Overshoot: {metrics['overshoot_percent']:.1f}%")
        print(
            f"   Max velocity: {metrics['max_velocity']:.2f} rad/s "
            f"(violation: {metrics['velocity_violation_percent']:.1f}%)"
        )
        print(f"   RMS error: {metrics['rms_error_deg']:.3f}°")
        print(
            f"   Settling time: {metrics['settling_time']:.3f}s"
            if metrics["settling_time"]
            else "   Settling time: None"
        )

        # Pass/fail indicators
        issues = []
        if metrics["overshoot_percent"] > 10:
            issues.append(
                f"❌ Overshoot too high ({metrics['overshoot_percent']:.1f}%)"
            )
        if metrics["velocity_violation_percent"] > 0:
            issues.append(
                f"❌ Velocity violation ({metrics['velocity_violation_percent']:.1f}%)"
            )
        if metrics["rms_error_deg"] > 1.0:
            issues.append(f"⚠️  RMS error high ({metrics['rms_error_deg']:.3f}°)")

        if issues:
            for issue in issues:
                print(f"   {issue}")
        else:
            print("   ✅ All criteria met!")

    # Summary table
    print("\n" + "=" * 80)
    print("📊 COMPARISON TABLE")
    print("=" * 80)
    print(f"{'Config':<40} {'ζ':>6} {'OS%':>6} {'Vel':>6} {'RMS°':>7} {'Settle':>8}")
    print("-" * 80)

    for r in results:
        settling = f"{r['settling_time']:.2f}s" if r["settling_time"] else "N/A"
        print(
            f"{r['config']['name']:<40} "
            f"{r['zeta']:>6.3f} "
            f"{r['overshoot_percent']:>6.1f} "
            f"{r['max_velocity']:>6.2f} "
            f"{r['rms_error_deg']:>7.3f} "
            f"{settling:>8}"
        )

    # Find best configuration
    print("\n" + "=" * 80)
    print("🏆 BEST CONFIGURATION")
    print("=" * 80)

    # Filter for valid configs (no velocity violations, overshoot < 10%)
    valid = [
        r
        for r in results
        if r["velocity_violation_percent"] == 0
        and r["overshoot_percent"] < 10
        and r["config"]["use_improved"]
    ]

    if valid:
        # Sort by RMS error (lower is better)
        best = min(valid, key=lambda x: x["rms_error_deg"])
        print(f"\nChosen: {best['config']['name']}")
        print(
            f"  Gains: kp_pos={best['config']['kp_pos']}, "
            f"kp_vel={best['config']['kp_vel']}, "
            f"ki_vel={best['config']['ki_vel']}, "
            f"kd_vel={best['config']['kd_vel']}"
        )
        kff_v = best["config"].get("kff_vel", 1.0)
        kff_a = best["config"].get("kff_accel", 0.0)
        print(f"  Feedforward: kff_vel={kff_v}, kff_accel={kff_a}")
        print(f"  Overshoot: {best['overshoot_percent']:.1f}%")
        print(f"  Max velocity: {best['max_velocity']:.2f} rad/s")
        print(f"  RMS error: {best['rms_error_deg']:.3f}°")
        print(f"  Damping ratio: ζ = {best['zeta']:.3f}")
        print("\n✅ This configuration will be used for demo generation.")

        return best["config"]
    else:
        print("\n⚠️  No configuration met all criteria. Using Option 2 as fallback.")
        return test_configs[2]  # Option 2 as fallback


def main():
    """Generate demo data and reports."""
    print("=" * 60)
    print("FOC Test Visualization System - Demo")
    print("=" * 60)

    # First, tune controller gains
    best_config = tune_controller_gains()

    # Generate demo data
    print("\n🔧 Generating demo FOC telemetry data...")

    json_files = []
    json_files.append(
        simulate_trapezoidal_motion(
            use_improved_controller=best_config["use_improved"],
            kp_pos=best_config["kp_pos"],
            kp_vel=best_config["kp_vel"],
            ki_vel=best_config["ki_vel"],
            kd_vel=best_config["kd_vel"],
            kff_vel=best_config.get("kff_vel", 1.0),
            kff_accel=best_config.get("kff_accel", 0.0),
        )
    )
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
