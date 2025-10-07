"""
BLDC Motor Simulator for Renode
Simulates motor physics and dynamics for testing FOC control
"""

from Antmicro.Renode.Logging import *
import math

class MotorSimulator:
    """
    Simple BLDC motor physics simulation
    
    Models:
    - Inertia
    - Friction (viscous + coulomb)
    - Back-EMF
    - Torque generation
    - Load torque
    """
    
    def __init__(self):
        # Motor parameters
        self.kt = 0.1  # Torque constant (Nm/A)
        self.ke = 0.1  # Back-EMF constant (V/rad/s)
        self.r_phase = 1.0  # Phase resistance (Ohms)
        self.l_phase = 0.001  # Phase inductance (H)
        
        # Mechanical parameters
        self.inertia = 0.001  # kg⋅m²
        self.friction_viscous = 0.01  # Nm/(rad/s)
        self.friction_coulomb = 0.05  # Nm (static friction)
        
        # State
        self.position = 0.0  # rad
        self.velocity = 0.0  # rad/s
        self.acceleration = 0.0  # rad/s²
        
        self.torque_motor = 0.0  # Motor torque (Nm)
        self.torque_load = 0.0   # External load (Nm)
        
        self.logger = Logger.GetLogger("MotorSim")
        self.logger.Log(LogLevel.Info, "Motor simulator initialized")
    
    def Reset(self):
        """Reset motor state"""
        self.position = 0.0
        self.velocity = 0.0
        self.acceleration = 0.0
        self.torque_motor = 0.0
        self.torque_load = 0.0
    
    def SetCurrent(self, current_a, current_b, current_c):
        """
        Set phase currents and calculate torque
        
        Simplified: torque = kt * |I_total|
        """
        # Total current (simplified)
        i_total = math.sqrt(current_a**2 + current_b**2 + current_c**2)
        
        # Motor torque
        self.torque_motor = self.kt * i_total
    
    def SetLoad(self, torque_nm):
        """Set external load torque"""
        self.torque_load = torque_nm
        self.logger.Log(LogLevel.Info, f"Load torque set: {torque_nm:.3f}Nm")
    
    def Update(self, dt):
        """
        Update motor state
        
        Dynamics:
        τ_net = τ_motor - τ_load - τ_friction
        α = τ_net / J
        ω = ω + α⋅dt
        θ = θ + ω⋅dt
        """
        # Friction torque
        friction = self.friction_viscous * self.velocity
        if abs(self.velocity) < 0.01:
            # Static friction
            friction += math.copysign(self.friction_coulomb, self.velocity) if self.velocity != 0 else 0
        
        # Net torque
        torque_net = self.torque_motor - self.torque_load - friction
        
        # Acceleration
        self.acceleration = torque_net / self.inertia
        
        # Integrate velocity
        self.velocity += self.acceleration * dt
        
        # Integrate position
        self.position += self.velocity * dt
        
        # Wrap position to [-π, π]
        while self.position > math.pi:
            self.position -= 2 * math.pi
        while self.position < -math.pi:
            self.position += 2 * math.pi
    
    def GetPosition(self):
        """Get current position (rad)"""
        return self.position
    
    def GetVelocity(self):
        """Get current velocity (rad/s)"""
        return self.velocity
    
    def GetAcceleration(self):
        """Get current acceleration (rad/s²)"""
        return self.acceleration
    
    def GetBackEMF(self):
        """Calculate back-EMF voltage"""
        return self.ke * abs(self.velocity)

