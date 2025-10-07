# 🔌 Renode Mock Peripherals

## Overview

Mock peripherals для полноценной эмуляции CLN17 v2.0 Joint Controller hardware в Renode.

---

## 📦 Available Peripherals

### 1. AS5047P Magnetic Encoder (`as5047p_encoder.py`)

**Type:** SPI Peripheral  
**Resolution:** 14-bit (16384 counts/revolution)  
**Interface:** SPI (connected to `spi1`)

**Features:**
- Absolute position measurement
- Velocity calculation
- Error flag simulation
- Configurable direction
- Zero position offset

**Commands:**
```python
encoder = AS5047PEncoder()
encoder.SetPosition(angle_counts)  # Set position (0-16383)
encoder.SetVelocity(vel_rad_s)     # Set velocity for simulation
position = encoder.Transmit(0x3FFF)  # Read position via SPI
```

**Usage in Tests:**
```robot
# Set encoder to 45 degrees
Execute Command    encoder SetPosition 4096

# Read position via SPI (firmware does this automatically)
${pos} =    Read Encoder Position
Should Be Equal    ${pos}    4096
```

---

### 2. Current Sense ADC (`current_sense_adc.py`)

**Type:** ADC Mock  
**Resolution:** 12-bit (4096 counts)  
**Channels:** 3 (Phase A, Phase B, Vbus)

**Features:**
- 3-phase current measurement
- DC bus voltage monitoring
- Configurable gain and offset
- Load torque simulation
- Realistic ADC conversion

**Commands:**
```python
adc = CurrentSenseADC()
adc.SetPhaseCurrent('A', current_amps)  # Set phase current
adc.SetLoad(torque_nm)                   # Set load (auto-calculates currents)
counts = adc.ReadChannel(0)              # Read Phase A
```

**Usage in Tests:**
```robot
# Simulate 2A motor current
Execute Command    currentADC SetLoad 0.2

# Read current (firmware does this)
${current} =    Get Motor Current
Should Be True    ${current} > 1.5
```

---

### 3. CAN Test Device (`can_test_device.py`)

**Type:** CAN Device (External Master)  
**Protocol:** iRPC over CAN-FD  
**ID:** 0x00 (Host/Master)

**Features:**
- Send iRPC commands
- Receive responses and telemetry
- Frame logging
- Automatic payload encoding
- Response parsing

**Commands:**
```python
canTest = CANTestDevice()
canTest.SendConfigure()                          # Send Configure command
canTest.SendActivate()                           # Send Activate command
canTest.SendSetTarget(90.0, 150.0)              # SetTarget (angle°, vel°/s)
canTest.SendSetTargetV2(90, 100, 500, 0)        # SetTargetV2 (trap profile)
response = canTest.GetLastResponse()             # Get response frame
payload_type = canTest.GetResponsePayloadType()  # Get payload type
```

**Usage in Tests:**
```robot
# Send Configure command
Execute Command    canTest SendConfigure

# Wait for Ack response
Wait For CAN Response    Ack    timeout=1s

# Send motion command
Execute Command    canTest SendSetTargetV2 90.0 100.0 500.0 0

# Verify response
${resp} =    Get CAN Response
Should Be Equal    ${resp.payload_type}    Ack
```

---

### 4. Motor Simulator (`motor_simulator.py`)

**Type:** Physics Simulation  
**Model:** BLDC Motor with Inertia & Friction

**Features:**
- Motor physics simulation
- Inertia modeling (J = 0.001 kg⋅m²)
- Viscous + Coulomb friction
- Back-EMF calculation
- Torque from current
- Load torque application

**Parameters:**
```python
# Motor constants
kt = 0.1      # Torque constant (Nm/A)
ke = 0.1      # Back-EMF constant (V/rad/s)
J = 0.001     # Inertia (kg⋅m²)
b = 0.01      # Viscous friction (Nm/(rad/s))
Tc = 0.05     # Coulomb friction (Nm)
```

**Commands:**
```python
motor = MotorSimulator()
motor.SetCurrent(ia, ib, ic)     # Set phase currents
motor.SetLoad(torque_nm)          # Apply load torque
motor.Update(dt)                  # Update physics (called automatically)
pos = motor.GetPosition()         # Get position (rad)
vel = motor.GetVelocity()         # Get velocity (rad/s)
```

**Usage in Tests:**
```robot
# Apply 0.5 Nm load
Execute Command    motor SetLoad 0.5

# Run motion profile
Send Motion Command    90.0

# Wait for motion complete
Wait For Motion Complete

# Verify position reached despite load
${pos} =    Get Motor Position
Should Be Equal    ${pos}    90.0    tolerance=2.0
```

---

## 🔗 Integration with Firmware

### Data Flow

```
[CAN Test Device] --iRPC--> [FDCAN1] --> [Firmware]
                                              |
                                              v
                                    [Motion Planner]
                                              |
                                              v
                                    [FOC Controller]
                                              |
                    +-------------------------+-------------------------+
                    |                         |                         |
                    v                         v                         v
              [Motor Simulator]          [Encoder]              [Current ADC]
                    |                         |                         |
                    v                         v                         v
              Position/Vel              Position              Phase Currents
                    |                         |                         |
                    +-------------------------+-------------------------+
                                              |
                                              v
                                        [Telemetry]
                                              |
                                              v
                    [FDCAN1] <--iRPC-- [CAN Test Device]
```

### Update Loop

```python
# Renode simulation loop (simplified)
while running:
    # 1. Firmware reads encoder
    encoder_pos = spi1.Transfer(READ_ANGLE)
    
    # 2. Firmware reads currents
    current_a = adc1.ReadChannel(0)
    current_b = adc1.ReadChannel(1)
    
    # 3. FOC calculates PWM
    pwm_a, pwm_b, pwm_c = foc_controller.Update()
    
    # 4. Motor simulator receives currents
    motor.SetCurrent(pwm_to_current(pwm_a, pwm_b, pwm_c))
    motor.Update(dt)
    
    # 5. Encoder reflects motor position
    encoder.SetPosition(motor.GetPosition())
    
    # 6. ADC reflects motor currents
    currentADC.SetPhaseCurrent('A', motor.current_a)
```

---

## 🧪 Usage in Tests

### Example: Motion Planning Test

```robot
*** Test Cases ***
Should Execute Trapezoidal Profile
    [Documentation]    Test trapezoidal motion profile with load
    
    # Setup
    Load Firmware    @firmware.elf
    Start Emulation
    
    # Initialize peripherals
    Execute Command    encoder SetPosition 0
    Execute Command    motor SetLoad 0.3
    Execute Command    currentADC SetLoad 0.3
    
    # Connect CAN device
    Execute Command    canTest SendConfigure
    Wait For CAN Response    Ack
    
    Execute Command    canTest SendActivate  
    Wait For CAN Response    Ack
    
    # Send motion command
    Execute Command    canTest SendSetTargetV2 90.0 100.0 500.0 0
    Wait For CAN Response    Ack
    
    # Wait for motion complete
    Sleep    2s
    
    # Verify position reached
    ${motor_pos} =    Execute Command    motor GetPosition
    ${motor_pos_deg} =    Evaluate    ${motor_pos} * 180 / 3.14159
    Should Be True    abs(${motor_pos_deg} - 90.0) < 2.0
    
    # Verify load was handled
    ${current} =    Execute Command    currentADC ReadChannel 0
    Should Be True    ${current} > 100  # Some current drawn
```

---

## 🎯 Test Scenarios

### 1. Basic Motion
- Zero load
- Perfect tracking
- Nominal parameters

### 2. Load Scenarios
- Constant load torque
- Variable load
- Stall conditions

### 3. Adaptive Control
- coolStep activation (low load)
- dcStep activation (high load)
- stallGuard detection

### 4. Fault Injection
- Encoder errors
- Overcurrent
- Communication loss

---

## 📊 Performance Metrics

### Timing
- FOC loop: 10 kHz (100 µs)
- Encoder read: ~5 µs
- ADC read: ~2 µs
- CAN TX/RX: ~20 µs

### Accuracy
- Position: ±0.022° (14-bit resolution)
- Current: ±10 mA (12-bit ADC)
- Timing: ±1 µs (simulation quantum)

---

## 🔧 Configuration

### Encoder Settings
```python
encoder.zero_position = 0      # Zero position offset
encoder.direction = 1          # 1 = forward, -1 = reverse
```

### Motor Settings
```python
motor.inertia = 0.001         # kg⋅m² (adjust for load)
motor.friction_viscous = 0.01  # Nm/(rad/s)
motor.friction_coulomb = 0.05  # Nm
```

### ADC Settings
```python
currentADC.current_gain = 50   # mV/A (amplifier gain)
currentADC.current_offset = 1.65  # V (half Vref)
```

---

## 💡 Tips

1. **Start Simple:** Test basic motion with zero load first
2. **Add Load Gradually:** Increase load incrementally
3. **Check Logs:** Enable debug logging to see peripheral activity
4. **Use Breakpoints:** Pause emulation to inspect state
5. **Verify Timing:** Check that FOC loop runs at 10 kHz

---

## 🐛 Debugging

### Check Peripheral State
```robot
# Encoder
${pos} =    Execute Command    encoder GetPosition
Log    Encoder position: ${pos}

# Motor
${vel} =    Execute Command    motor GetVelocity
Log    Motor velocity: ${vel} rad/s

# ADC
${current} =    Execute Command    currentADC ReadChannel 0
Log    Phase A current: ${current} counts
```

### Enable Detailed Logging
```robot
Execute Command    encoder SetLogLevel Debug
Execute Command    motor SetLogLevel Debug
Execute Command    canTest SetLogLevel Debug
```

---

## 🎉 Benefits

✅ **Realistic:** Physics-based motor simulation  
✅ **Complete:** Full peripheral emulation  
✅ **Debuggable:** Step through firmware in emulator  
✅ **Repeatable:** Deterministic test results  
✅ **Fast:** Faster than real hardware  
✅ **Safe:** No risk of hardware damage  

---

**Готово к тестированию!** 🚀

Peripherals готовы, platform настроен, можно запускать E2E тесты!

