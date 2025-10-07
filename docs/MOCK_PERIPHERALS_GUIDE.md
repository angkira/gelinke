# 🎭 Mock Peripherals Guide

Руководство по использованию Python mock peripherals для тестирования в Renode.

---

## 📋 Обзор

**Python mock peripherals** позволяют тестировать прошивку **БЕЗ изменения кода firmware**!

### Что Моки Делают

| Mock Peripheral | Эмулирует | Для Чего |
|-----------------|-----------|----------|
| **CAN Device Mock** | Внешнее CAN-устройство (arm) | Отправка iRPC команд |
| **ADC Mock** | Токовые сенсоры (3 фазы) | FOC control loop testing |
| **Encoder Mock** | TLE5012B энкодер | Angle/velocity reading |

---

## 🚀 Quick Start

### 1. Используй Platform с Моками

```robot
*** Settings ***
Resource    test_helpers.robot

*** Variables ***
${PLATFORM}    ${CURDIR}/../stm32g431cb_with_mocks.repl

*** Test Cases ***
My Test With Mocks
    Execute Command    machine LoadPlatformDescription @${PLATFORM}
    # ... моки доступны!
```

### 2. Управляй Моками из Тестов

```robot
*** Test Cases ***
Test FOC With Synthetic Motor
    # Setup
    Execute Command    machine LoadPlatformDescription @${PLATFORM}
    Execute Command    sysbus LoadELF $elf
    Start Emulation
    
    # Set motor running conditions
    Setup Running Motor Conditions    velocity_deg_s=30.0    current_amps=2.0
    
    # Wait and verify
    Sleep    1s
    ${angle}=    Read Encoder Angle
    Should Be True    ${angle} > 20.0
```

---

## 🎛️ CAN Device Mock

### Описание

Эмулирует внешнее CAN-устройство (например, arm controller), которое отправляет iRPC команды в firmware.

### Register Map

```
0x00 (RW): Control register
    Read:  Has response flag (1 if response received)
    Write: Clear RX queue
0x04 (R):  Message count in RX queue
```

### Примеры Использования

#### Отправить Configure Command

```robot
Send CAN Configure Command
Wait For Line On Uart    Received iRPC message    timeout=2
```

#### Отправить SetTarget Command

```robot
Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
Sleep    0.5s
# Verify motor is moving toward target
```

#### Проверить Response

```robot
Send CAN Activate Command
Check CAN Response Received
# Response was received from firmware
```

### Python API

```python
# In Renode monitor or Python peripheral
canDeviceMock = self.machine['sysbus.canDeviceMock']

# Send commands
msg_id = canDeviceMock.send_configure()
msg_id = canDeviceMock.send_activate()
msg_id = canDeviceMock.send_set_target(90.0, 150.0)

# Check responses
has_response = canDeviceMock.has_response()
last_response = canDeviceMock.get_last_response()
```

---

## 📊 ADC Mock

### Описание

Эмулирует токовые сенсоры для 3-фазного двигателя. Может генерировать синтетические синусоидальные токи или устанавливать произвольные значения.

### Register Map

```
0x00 (RW): Phase A current (raw ADC, 0-4095, offset 2048)
0x04 (RW): Phase B current (raw ADC, 0-4095, offset 2048)
0x08 (RW): Phase C current (raw ADC, 0-4095, offset 2048)
0x0C (RW): DC bus voltage (raw ADC, 0-4095)
0x10 (RW): Control register
    Bit 0: Enable synthetic motion (generates 3-phase sine waves)
```

### Примеры Использования

#### Установить Токи Вручную

```robot
# Set 2A on phase A
Set ADC Phase Current    A    2.0

# Set -1A on phase B
Set ADC Phase Current    B    -1.0

# Set zero current on phase C
Set ADC Phase Current    C    0.0
```

#### Включить Synthetic Motion

```robot
# Simulate motor running at 1 rad/s with 2A amplitude
Enable ADC Synthetic Motion    velocity_rad_s=1.0    amplitude_amps=2.0

# Run for some time
Sleep    2s

# Disable motion
Disable ADC Synthetic Motion
```

#### Инжектировать Overcurrent

```robot
# Inject 20A on phase A (triggers fault)
Inject ADC Overcurrent    phase=A

# Verify fault was detected
Wait For Line On Uart    Overcurrent detected    timeout=1
```

#### Установить DC Voltage

```robot
# Set 48V DC bus
Set ADC DC Voltage    48.0

# Set low voltage (triggers undervoltage)
Set ADC DC Voltage    8.0
```

### Формулы

**ADC Value Calculation:**
```
Shunt resistance:  10 mOhm
Amplifier gain:    20 V/V
→ Voltage = Current × 0.2 V/A

ADC value = offset + (Voltage / 3.3V) × 4095
offset = 2048 (mid-scale 12-bit)
```

**Example:**
- Current = 2.0 A
- Voltage = 2.0 × 0.2 = 0.4 V
- ADC counts = 0.4 / 3.3 × 4095 ≈ 496
- ADC value = 2048 + 496 = 2544

---

## 🎯 Encoder Mock

### Описание

Эмулирует TLE5012B магнитный энкодер. Может устанавливать произвольный угол или симулировать непрерывное вращение.

### Register Map

```
0x00 (RW): Current angle (0-32767, 15-bit)
0x04 (RW): Angular velocity (millidegrees/sec, signed 32-bit)
0x08 (RW): Control register
    Bit 0: Enable motion (continuous rotation)
    Bit 1: Inject error on next read
0x0C (RW): Error injection type
    0 = No error
    1 = Bad CRC
    2 = Timeout (no response)
    3 = Invalid data
```

### Примеры Использования

#### Установить Угол

```robot
# Set encoder to 90 degrees
Set Encoder Angle    90.0

# Read it back
${angle}=    Read Encoder Angle
Should Be Equal As Numbers    ${angle}    90.0    delta=1.0
```

#### Включить Вращение

```robot
# Start rotating at 30 degrees/second
Enable Encoder Motion    velocity_deg_s=30.0

# Wait 3 seconds
Sleep    3s

# Angle should be ~90 degrees now
${angle}=    Read Encoder Angle
Should Be True    ${angle} > 80.0
```

#### Инжектировать Ошибку

```robot
# Inject CRC error
Inject Encoder Error    error_type=1

# Next SPI read will fail
Wait For Line On Uart    Encoder CRC error    timeout=1

# Clear error for subsequent reads
Clear Encoder Error
```

#### Wait for Target Angle

```robot
# Start rotation
Enable Encoder Motion    velocity_deg_s=45.0

# Wait until encoder reaches 180° (±5° tolerance)
Wait For Encoder Angle    target_deg=180.0    tolerance_deg=5.0    timeout_sec=5.0
```

### Формулы

**Angle Conversion:**
```
15-bit raw value: 0-32767
Degrees: 0-360

angle_deg = (raw_value / 32767.0) × 360.0
raw_value = (angle_deg / 360.0) × 32767
```

**Velocity:**
```
Stored in millidegrees/sec
velocity_deg_s = raw_value / 1000.0
```

---

## 🎬 Готовые Сценарии

### Nominal Operating Conditions

```robot
Setup Nominal Operating Conditions
# Sets:
# - Zero current on all phases
# - 48V DC bus
# - Encoder at 0°, stopped
```

### Running Motor Simulation

```robot
Setup Running Motor Conditions    velocity_deg_s=30.0    current_amps=2.0
# Sets:
# - Synthetic 3-phase currents (2A amplitude)
# - Encoder rotating at 30°/s
# - 48V DC bus
```

---

## 📝 Примеры Тестов

### Test 1: ADC Calibration

```robot
*** Test Cases ***
Should Calibrate ADC Offsets
    [Documentation]    Test ADC zero-current calibration
    
    # Setup
    Execute Command    machine LoadPlatformDescription @${PLATFORM}
    Execute Command    sysbus LoadELF $elf
    Create Terminal Tester    ${UART}
    Start Emulation
    
    # Set zero current (calibration condition)
    Setup Nominal Operating Conditions
    
    # Trigger calibration (via iRPC or internal trigger)
    Send CAN Configure Command
    
    # Wait for calibration
    Wait For Line On Uart    ADC calibration complete    timeout=5
    
    # Verify offsets are near 2048
    ${phase_a}=    Read ADC Phase Current    A
    Should Be True    ${phase_a} > 2000 and ${phase_a} < 2100
```

### Test 2: Overcurrent Detection

```robot
*** Test Cases ***
Should Detect Overcurrent And Stop Motor
    [Documentation]    Test overcurrent protection
    
    # Setup running motor
    Execute Command    machine LoadPlatformDescription @${PLATFORM}
    Execute Command    sysbus LoadELF $elf
    Create Terminal Tester    ${UART}
    Start Emulation
    
    Setup Running Motor Conditions    velocity_deg_s=30.0    current_amps=2.0
    Sleep    1s
    
    # Inject overcurrent
    Inject ADC Overcurrent    phase=A
    
    # Verify fault detected and PWM disabled
    Wait For Line On Uart    Overcurrent detected    timeout=1
    Wait For Line On Uart    PWM disabled            timeout=1
    Wait For Line On Uart    State: Fault            timeout=1
```

### Test 3: Position Tracking

```robot
*** Test Cases ***
Should Track Position Setpoint
    [Documentation]    Test position controller
    
    # Setup
    Execute Command    machine LoadPlatformDescription @${PLATFORM}
    Execute Command    sysbus LoadELF $elf
    Create Terminal Tester    ${UART}
    Start Emulation
    
    # Activate joint
    Send CAN Configure Command
    Send CAN Activate Command
    Sleep    0.5s
    
    # Set target position
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    
    # Enable encoder motion (simulates motor response)
    Enable Encoder Motion    velocity_deg_s=30.0
    
    # Wait for target
    Wait For Encoder Angle    target_deg=90.0    tolerance_deg=5.0    timeout_sec=5.0
    
    # Verify reached target
    ${angle}=    Read Encoder Angle
    Should Be True    ${angle} > 85.0 and ${angle} < 95.0
```

---

## 🛠️ Architecture

### How It Works

```
┌─────────────────────────────────────────────────┐
│           Robot Framework Test                  │
│  "Set ADC Phase Current A 2.0"                  │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│        Test Helper Keyword (Robot)              │
│  Execute Command sysbus.adcMock WriteDoubleWord │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│      Python Peripheral (Renode)                 │
│  if request.isWrite: self.adc.phase_a = value   │
└────────────────┬────────────────────────────────┘
                 │
                 ▼
┌─────────────────────────────────────────────────┐
│         Firmware Reads Value                    │
│  let current = adc.read_channel(ADC_CHANNEL_A)  │
└─────────────────────────────────────────────────┘
```

### Integration Points

**ADC Mock → Firmware:**
- Firmware reads `sysbus.adc1` / `sysbus.adc2`
- Renode redirects to `adcMock` via platform configuration
- Mock returns synthetic values

**Encoder Mock → Firmware:**
- Firmware does SPI transfer to `sysbus.spi1`
- Renode connects SPI to `encoderMock`
- Mock responds with TLE5012B protocol

**CAN Device Mock → Firmware:**
- Mock sends CAN frames via CAN hub
- Firmware receives via `sysbus.fdcan1`
- Standard iRPC message flow

---

## 🎯 Next Steps

### Enable Mocks in Tests

1. **Update platform reference:**
```robot
${PLATFORM}    ${CURDIR}/../stm32g431cb_with_mocks.repl
```

2. **Import helpers:**
```robot
Resource    test_helpers.robot
```

3. **Use keywords:**
```robot
Setup Running Motor Conditions
```

### Activate Pending Tests

80 тестов ждут! Просто:
- ✅ Добавь `Resource test_helpers.robot`
- ✅ Используй mock keywords
- ✅ Убери `[Tags] future` или `Pass Execution`

---

## 📚 Reference

### Files

```
renode/
├── peripherals/
│   ├── can_device_mock.py       # CAN device simulator
│   ├── adc_mock.py              # Current sensors simulator
│   └── encoder_mock.py          # TLE5012B simulator
├── stm32g431cb_with_mocks.repl  # Platform with mocks
└── tests/
    └── test_helpers.robot       # Robot Framework keywords
```

### Documentation

- **This Guide:** `docs/MOCK_PERIPHERALS_GUIDE.md`
- **Test Suite:** `docs/TESTING_SUITE.md`
- **Implementation:** `docs/TEST_SUITE_IMPLEMENTATION.md`

---

## 🎉 Summary

**Python mock peripherals позволяют:**

✅ Тестировать прошивку БЕЗ изменений кода  
✅ Симулировать любые сенсоры и актуаторы  
✅ Инжектировать fault conditions  
✅ Проверять edge cases  
✅ Запускать все 100 тестов в Renode  

**Готово к использованию!** 🚀


