# CRITICAL: Firmware Does Not Match CLN17 V2.0 Hardware

**Status:** 🔴 **BLOCKING - WILL NOT WORK ON HARDWARE**
**Severity:** CRITICAL
**Date:** 2025-11-10
**Hardware Spec:** https://github.com/creapunk/TunePulse (Official)

---

## Executive Summary

The current firmware in this repository **DOES NOT MATCH** the CLN17 V2.0 hardware design. The firmware appears to be written for a different board configuration and will **NOT WORK** on actual CLN17 V2.0 hardware without major corrections.

### Critical Mismatches:

1. **PWM completely wrong** - Using TIM1 complementary instead of TIM2 independent
2. **All PWM pins wrong** - PA8/PA7/PA9/PA10/PB0/PB1 vs PA0/PA1/PB10/PB11
3. **Wrong motor driver type** - Code assumes 3-phase complementary, hardware has 2-phase H-bridge
4. **CAN pins wrong** - PA11/PA12 vs PB8/PB9
5. **Current sensing wrong** - PA0/PA1 vs PA3/PB0
6. **UART pins wrong** - PA9/PA10 vs PC10/PC11
7. **Encoder CS wrong** - PA4 vs PC4

**Estimated Rework:** 60-80% of hardware drivers need rewriting

---

## Detailed Comparison

### 1. PWM Motor Control - COMPLETELY WRONG

#### What Firmware Implements (WRONG):
```rust
// src/firmware/drivers/pwm.rs:29-34
let ch1 = PwmPin::new(p.PA8, OutputType::PushPull);    // Phase A High
let ch1n = ComplementaryPwmPin::new(p.PA7, ...);       // Phase A Low
let ch2 = PwmPin::new(p.PA9, ...);                     // Phase B High
let ch2n = ComplementaryPwmPin::new(p.PB0, ...);       // Phase B Low
let ch3 = PwmPin::new(p.PA10, ...);                    // Phase C High
let ch3n = ComplementaryPwmPin::new(p.PB1, ...);       // Phase C Low

ComplementaryPwm::new(p.TIM1, ..., CountingMode::CenterAlignedBothInterrupts);
pwm.set_dead_time(100); // Dead-time for complementary outputs
```

**Architecture:** 3-phase FOC with complementary PWM pairs
**Timer:** TIM1 with complementary outputs
**Pins:** PA8/PA7, PA9/PB0, PA10/PB1
**Driver Type:** 3-phase gate driver (e.g., DRV8305, IR2301)

#### What Hardware Actually Has (CORRECT):
```
// Actual CLN17 V2.0 hardware
Motor Driver: DRV8844 (2-phase H-bridge stepper driver)
Timer: TIM2 (4 independent channels)
Pins:
  - PA0:  TIM2_CH1 → DRV8844 AIN1
  - PA1:  TIM2_CH2 → DRV8844 AIN2
  - PB11: TIM2_CH4 → DRV8844 BIN1
  - PB10: TIM2_CH3 → DRV8844 BIN2
```

**Architecture:** 2-phase stepper with independent H-bridge control
**Timer:** TIM2 with 4 independent channels
**Pins:** PA0, PA1, PB10, PB11
**Driver Type:** DRV8844 H-bridge stepper driver

#### Why This Is Critical:
- **TIM1 vs TIM2:** Different timer peripheral entirely
- **Complementary vs Independent:** Completely different PWM mode
- **3-phase vs 2-phase:** Wrong motor control algorithm
- **All pins wrong:** Not a single PWM pin matches
- **Dead-time not needed:** DRV8844 has internal protection

**Impact:** Motor will not run at all. PWM outputs go to wrong pins.

---

### 2. Current Sensing - WRONG PINS

#### Firmware (WRONG):
```rust
// src/firmware/drivers/adc.rs (implied from pwm.rs usage)
Current A: PA0 (ADC1_IN1)
Current B: PA1 (ADC1_IN2)
```

#### Hardware (CORRECT):
```
Current A: PA3 (ADC1_IN4)  ← DRV8844 AISEN output
Current B: PB0 (ADC1_IN15) ← DRV8844 BISEN output
Vbus:      PA2 (ADC1_IN3)  ← Supply voltage monitoring
```

**Impact:**
- PA0/PA1 are PWM outputs, not ADC inputs!
- Reading current from PWM pins will give garbage data
- No bus voltage monitoring

---

### 3. CAN Communication - WRONG PINS

#### Firmware (WRONG):
```rust
// src/firmware/system.rs:106-107
p.FDCAN1,
p.PA12,    // TX
p.PA11,    // RX
```

#### Hardware (CORRECT):
```
CAN RX: PB8  (FDCAN1_RX)
CAN TX: PB9  (FDCAN1_TX)
CAN Shutdown: PA9 (GPIO, transceiver control)
CAN S: PA10 (GPIO, transceiver mode)
```

**Impact:**
- CAN communication will not work
- PA11/PA12 are USB pins, not CAN
- Missing transceiver control signals

---

### 4. UART Debug - WRONG PERIPHERAL AND PINS

#### Firmware (WRONG):
```rust
// src/firmware/system.rs:48-51
let uart = Uart::new(
    p.USART1,
    p.PA10, // RX
    p.PA9,  // TX
    ...
);
```

#### Hardware (CORRECT):
```
UART: USART3
TX: PC10 (USART3_TX)
RX: PC11 (USART3_RX)
```

**Impact:**
- UART debug will not work
- PA9/PA10 are CAN transceiver control, not UART
- Wrong USART peripheral

---

### 5. Encoder SPI - CS PIN WRONG

#### Firmware (WRONG):
```rust
// src/firmware/drivers/sensors.rs:39
let cs = Output::new(p.PA4, ...);  // Chip select
```

#### Hardware (CORRECT):
```
SPI1_SCK:  PA5 ✓ Correct
SPI1_MISO: PA6 ✓ Correct
SPI1_MOSI: PA7 ✓ Correct
SPI1_CS:   PC4 ❌ WRONG (firmware uses PA4)
```

**Impact:**
- Encoder CS on wrong pin
- PA4 is actually motor driver enable signal

---

### 6. Step-Dir Interface - COMPLETELY MISSING

#### Firmware (MISSING):
```rust
// src/firmware/tasks/step_dir.rs:178-179
// TODO: Read step/dir inputs from GPIO
// TODO: Update PWM outputs based on position
```

#### Hardware (REQUIRED):
```
STEP Input:   PB5 (EXTI interrupt)
DIR Input:    PB4 (GPIO read)
ENABLE Input: PA8 (GPIO read)
ERROR Output: PB3 (GPIO output)
```

**Impact:** Step-Dir mode advertised but completely non-functional

---

### 7. Motor Safety - COMPLETELY MISSING

#### Firmware (MISSING):
No implementation

#### Hardware (REQUIRED):
```
Motor Enable: PA4 (GPIO output to DRV8844 nSLEEP)
Fault Input:  PB1 (GPIO input from DRV8844 nFAULT)
Driver Reset: PB2 (GPIO output to DRV8844 nRESET)
```

**Impact:** Cannot safely enable/disable motor, no fault detection

---

### 8. Status LEDs - COMPLETELY MISSING

#### Firmware (MISSING):
No implementation

#### Hardware (PRESENT):
```
Red LED:   PB13 (active low)
Green LED: PB14 (active low)
Blue LED:  PB15 (active low)
```

**Impact:** No visual status indication

---

### 9. Supply Voltage Monitoring - MISSING

#### Firmware (MISSING):
Not implemented in ADC driver

#### Hardware (PRESENT):
```
Vbus: PA2 (ADC1_IN3)
```

**Impact:** Cannot detect power supply issues

---

## Complete Pin Mismatch Table

| Function | Firmware Pin | Hardware Pin | Match | Critical |
|----------|--------------|--------------|-------|----------|
| PWM A1 | PA8 (TIM1_CH1) | PA0 (TIM2_CH1) | ❌ | 🔴 YES |
| PWM A2 | PA7 (TIM1_CH1N) | PA1 (TIM2_CH2) | ❌ | 🔴 YES |
| PWM B1 | PA9 (TIM1_CH2) | PB11 (TIM2_CH4) | ❌ | 🔴 YES |
| PWM B2 | PB0 (TIM1_CH2N) | PB10 (TIM2_CH3) | ❌ | 🔴 YES |
| PWM C1 | PA10 (TIM1_CH3) | N/A | ❌ | 🔴 YES |
| PWM C2 | PB1 (TIM1_CH3N) | N/A | ❌ | 🔴 YES |
| Current A | PA0 (ADC1_IN1) | PA3 (ADC1_IN4) | ❌ | 🔴 YES |
| Current B | PA1 (ADC1_IN2) | PB0 (ADC1_IN15) | ❌ | 🔴 YES |
| Vbus | Not impl | PA2 (ADC1_IN3) | ❌ | 🟡 |
| Motor Enable | Not impl | PA4 (GPIO) | ❌ | 🔴 YES |
| SPI SCK | PA5 | PA5 | ✅ | |
| SPI MISO | PA6 | PA6 | ✅ | |
| SPI MOSI | PA7 | PA7 | ✅ | |
| SPI CS | PA4 | PC4 | ❌ | 🔴 YES |
| CAN RX | PA11 | PB8 | ❌ | 🔴 YES |
| CAN TX | PA12 | PB9 | ❌ | 🔴 YES |
| CAN SHDN | Not impl | PA9 | ❌ | 🟡 |
| CAN S | Not impl | PA10 | ❌ | 🟡 |
| USB D- | Conflict | PA11 | ⚠️ | |
| USB D+ | Conflict | PA12 | ⚠️ | |
| UART TX | PA9 (USART1) | PC10 (USART3) | ❌ | 🟡 |
| UART RX | PA10 (USART1) | PC11 (USART3) | ❌ | 🟡 |
| STEP | Not impl | PB5 | ❌ | 🔴 YES |
| DIR | Not impl | PB4 | ❌ | 🔴 YES |
| ENABLE | Not impl | PA8 | ❌ | 🟡 |
| ERROR | Not impl | PB3 | ❌ | 🟡 |
| Fault Input | Not impl | PB1 | ❌ | 🔴 YES |
| Driver Reset | Not impl | PB2 | ❌ | 🟡 |
| LED Red | Not impl | PB13 | ❌ | |
| LED Green | Not impl | PB14 | ❌ | |
| LED Blue | Not impl | PB15 | ❌ | |

**Legend:**
- 🔴 Critical - Will prevent hardware from working
- 🟡 High - Required for full functionality
- ✅ Correct

**Pins Correct:** 3 out of 30+ (10%)
**Critical Errors:** 13

---

## Root Cause Analysis

### Why This Happened:

1. **Firmware written for different board**
   - Code appears to be for a 3-phase FOC driver
   - Uses complementary PWM typical of BLDC gate drivers
   - Does not match DRV8844 H-bridge architecture

2. **No hardware reference during development**
   - Code written without CLN17 V2.0 schematic
   - Pin assignments guessed or copied from different board
   - No validation against actual hardware

3. **Wrong driver assumptions**
   - Assumed 3-phase BLDC motor
   - Assumed complementary PWM gate driver
   - Assumed different MCU pinout

---

## Required Changes - Priority Order

### Phase 1: Critical Fixes (BLOCKING)

**1. Rewrite PWM Driver** (CRITICAL - 8 hours)
- [ ] Change from TIM1 to TIM2
- [ ] Change from complementary to independent PWM
- [ ] Remap all pins: PA0, PA1, PB10, PB11
- [ ] Remove dead-time configuration
- [ ] Implement 4-channel control for DRV8844
- [ ] Test: Verify PWM output on oscilloscope

**2. Fix ADC Current Sensing** (CRITICAL - 2 hours)
- [ ] Remap Current A: PA0 → PA3
- [ ] Remap Current B: PA1 → PB0 (verify correct)
- [ ] Add Vbus monitoring: PA2
- [ ] Update ADC channel configuration
- [ ] Update DMA mappings
- [ ] Test: Verify ADC readings

**3. Fix CAN Pins** (CRITICAL - 2 hours)
- [ ] Remap CAN RX: PA11 → PB8
- [ ] Remap CAN TX: PA12 → PB9
- [ ] Add CAN transceiver control: PA9 (SHDN), PA10 (S)
- [ ] Update FDCAN HAL configuration
- [ ] Test: Verify CAN communication

**4. Add Motor Enable/Fault** (CRITICAL - 2 hours)
- [ ] Add PA4 GPIO output (motor enable)
- [ ] Add PB1 GPIO input (fault detection)
- [ ] Add PB2 GPIO output (driver reset)
- [ ] Implement enable/disable logic
- [ ] Add fault interrupt handler
- [ ] Test: Verify motor enable/disable

**5. Fix Encoder CS Pin** (CRITICAL - 1 hour)
- [ ] Remap CS: PA4 → PC4
- [ ] Verify SPI communication
- [ ] Test: Read encoder position

### Phase 2: High Priority Features (4-8 hours)

**6. Implement Step-Dir GPIO** (HIGH - 4 hours)
- [ ] Add PB5 EXTI (STEP input)
- [ ] Add PB4 GPIO (DIR input)
- [ ] Add PA8 GPIO (ENABLE input)
- [ ] Add PB3 GPIO (ERROR output)
- [ ] Implement EXTI interrupt handler
- [ ] Connect to StepDirController
- [ ] Test: Step pulse response

**7. Fix UART Debug** (HIGH - 2 hours)
- [ ] Change USART1 → USART3
- [ ] Remap TX: PA9 → PC10
- [ ] Remap RX: PA10 → PC11
- [ ] Update DMA channels
- [ ] Test: Verify UART output

**8. Add Status LEDs** (MEDIUM - 1 hour)
- [ ] Add PB13 GPIO (red LED)
- [ ] Add PB14 GPIO (green LED)
- [ ] Add PB15 GPIO (blue LED)
- [ ] Implement status indication
- [ ] Test: LED control

### Phase 3: Motor Control Algorithm (8-16 hours)

**9. Update FOC for 2-Phase** (HIGH - 8 hours)
- [ ] Review DRV8844 control strategy
- [ ] Implement 2-phase FOC (if applicable)
- [ ] Or switch to stepper control algorithm
- [ ] Update current control loops
- [ ] Test: Motor motion

**10. Complete Step-Dir Implementation** (HIGH - 4 hours)
- [ ] Connect GPIO to controller
- [ ] Test step pulse response
- [ ] Verify direction control
- [ ] Test at various step frequencies

---

## Testing Strategy

### Before Hardware Testing:

1. **Pin Verification** (1 hour)
   - Print pin mappings to UART
   - Cross-reference with schematic
   - Create pin verification test

2. **Renode Update** (2 hours)
   - Update .repl file with correct pins
   - Update mock peripherals
   - Re-run all tests

3. **Static Analysis** (1 hour)
   - Verify no pin conflicts
   - Check peripheral clock enables
   - Validate DMA channel assignments

### Hardware Bring-Up Procedure:

1. **Power-On Test**
   - Verify 3.3V rail
   - Check LED blink (if implemented)
   - Monitor current consumption

2. **Communication Test**
   - UART output verification
   - CAN bus communication
   - USB enumeration

3. **Sensor Test**
   - Encoder position reading
   - Current sense validation
   - Vbus monitoring

4. **Motor Test**
   - Enable driver
   - Apply small PWM duty
   - Verify no faults
   - Test current limiting

5. **Full System Test**
   - FOC closed-loop control
   - Step-Dir interface
   - Fault handling
   - Thermal management

---

## Estimated Timeline

| Phase | Description | Time | Status |
|-------|-------------|------|--------|
| 1 | Critical pin fixes | 15 hours | 🔴 Not Started |
| 2 | Feature completion | 7 hours | 🔴 Not Started |
| 3 | Algorithm updates | 12 hours | 🔴 Not Started |
| 4 | Testing & validation | 8 hours | 🔴 Not Started |
| **Total** | | **42 hours** | **~1 week** |

---

## Recommendations

### Immediate Actions:

1. **Stop current development** until pins are fixed
2. **Create pin mapping module** as single source of truth
3. **Update Renode platform** with correct pins
4. **Re-run all tests** after fixes

### Process Improvements:

1. **Hardware-first development**
   - Always start with schematic
   - Verify pins before writing code
   - Test on hardware early

2. **Documentation**
   - Maintain hardware pinout document
   - Link code to schematic references
   - Update docs with hardware changes

3. **Validation**
   - Automated pin conflict detection
   - Hardware validation tests
   - Continuous hardware-in-loop testing

---

## References

- **Hardware Schematic:** https://github.com/creapunk/CLN-ClosedLoopNemaDriver/tree/main/hardware/CLN17/V2.0
- **Official Firmware:** https://github.com/creapunk/TunePulse/tree/main/src/target/cln17_v2_0
- **Official Pinout:** [CLN17_V2_HARDWARE_PINOUT.md](CLN17_V2_HARDWARE_PINOUT.md)
- **DRV8844 Datasheet:** Texas Instruments

---

**CRITICAL WARNING:** Do NOT attempt to run current firmware on CLN17 V2.0 hardware. The pin mismatches will cause:
- PWM outputs on wrong pins
- Potential short circuits
- Current sense errors
- Communication failures
- Motor control failure

**All critical pins must be corrected before hardware testing.**

---

**Document Version:** 1.0
**Status:** CRITICAL BLOCKING ISSUE
**Priority:** P0 - Immediate Action Required
**Last Updated:** 2025-11-10
