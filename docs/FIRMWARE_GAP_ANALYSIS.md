# CLN17 v2.0 Firmware Gap Analysis

**Date:** 2025-11-10
**Firmware Version:** CLN17 v2.0 Joint Controller
**Target MCU:** STM32G431CB
**Analysis Status:** Critical issues identified

---

## Executive Summary

This document identifies critical gaps and issues in the CLN17 v2.0 firmware development, particularly focusing on pin configuration conflicts and missing hardware interfaces. The firmware has a solid foundation with FOC and Step-Dir control, but several hardware integration issues must be resolved before deployment.

### Critical Findings:
1. **Pin Conflict:** PA9/PA10 used by both UART and PWM (system.rs:50-51 vs pwm.rs:31-33)
2. **Missing Step-Dir GPIO:** Step/Direction inputs not implemented (step_dir.rs:178-179)
3. **No Motor Safety Pins:** Missing enable/fault control
4. **Missing DC Bus Monitoring:** No voltage sensing for power supply health
5. **No Hardware Documentation:** Pin mappings exist only in code

---

## 1. Critical Pin Conflicts

### Issue #1: UART vs PWM Conflict on PA9/PA10

**Severity:** CRITICAL - Hardware Blocking

**Problem:**
PA9 and PA10 are configured for BOTH USART1 debug output AND TIM1 PWM output, creating a hardware conflict.

**Evidence:**

**File:** `src/firmware/system.rs:43-56`
```rust
// Initialize UART for test logging (USART1: PA9=TX, PA10=RX)
defmt::info!("[TRACE] UART pins: PA9=TX, PA10=RX");

let mut uart = Uart::new(
    p.USART1,
    p.PA10, // RX
    p.PA9,  // TX
    UartIrqs,
    p.DMA1_CH1,  // TX DMA
    p.DMA1_CH2,  // RX DMA
    uart_log::uart_config(),
).expect("UART init failed");
```

**File:** `src/firmware/drivers/pwm.rs:31-33`
```rust
let ch2 = PwmPin::new(p.PA9, OutputType::PushPull);   // Phase B High
let ch2n = ComplementaryPwmPin::new(p.PB0, OutputType::PushPull);
let ch3 = PwmPin::new(p.PA10, OutputType::PushPull);  // Phase C High
let ch3n = ComplementaryPwmPin::new(p.PB1, OutputType::PushPull);
```

**Impact:**
- Cannot use both UART debug and 3-phase PWM simultaneously
- Motor control will fail if both are initialized
- Embassy peripheral ownership will panic at runtime

**Recommended Solutions:**

**Option A: Move UART to USART2 (Preferred)**
```rust
// Use PA2/PA3 for USART2
let uart = Uart::new(
    p.USART2,
    p.PA3,  // RX
    p.PA2,  // TX
    // ...
);
```

**Option B: Conditional Compilation**
```rust
#[cfg(feature = "debug-uart")]
{
    // Initialize UART for debug builds only
}
```

**Option C: Disable UART in Production**
- Remove UART initialization from system.rs
- Use defmt RTT for debugging only

---

### Issue #2: CAN vs USB Pin Sharing (Low Priority)

**Severity:** LOW - Future Planning

**Problem:**
PA11/PA12 shared between FDCAN1 (active) and USB (stub only).

**Current State:**
- FDCAN1 is actively used (system.rs:105-108)
- USB driver is stub only (drivers/usb.rs:24)

**Impact:**
- Cannot use both CAN and USB simultaneously
- Not currently blocking since USB is not implemented

**Recommendation:**
- Document that CAN-FD and USB are mutually exclusive
- Consider alternative designs if both are needed (USB hub chip, different MCU)

---

## 2. Missing Pin Configurations

### Priority 1: Step-Dir GPIO Inputs

**Severity:** HIGH - Feature Blocking

**Status:** Logic implemented, GPIO interface missing

**Evidence:**

**File:** `src/firmware/tasks/step_dir.rs:178-179`
```rust
loop {
    ticker.next().await;
    iteration = iteration.wrapping_add(1);

    // TODO: Read step/dir inputs from GPIO
    // TODO: Update PWM outputs based on position
}
```

**Required Pins:**
| Pin | Function | Type | Notes |
|-----|----------|------|-------|
| PA15 | STEP input | GPIO EXTI | Rising edge interrupt |
| PB3 | DIR input | GPIO Input | Direction control |
| PB4 | ENABLE input | GPIO Input | Optional, active high |

**Missing Implementation:**
1. GPIO EXTI configuration for step pulse detection
2. Interrupt handler to increment step counter
3. GPIO read for direction signal
4. Integration with StepDirController

**Impact:**
- Step-Dir mode cannot accept external step/dir signals
- Feature is advertised but non-functional
- Controller logic exists but has no input source

**Recommended Implementation:**
```rust
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::{Input, Pull};

#[embassy_executor::task]
pub async fn control_loop(
    step_pin: ExtiInput<'static, peripherals::PA15>,
    dir_pin: Input<'static, peripherals::PB3>,
) {
    let mut controller = StepDirController::new(&config);
    controller.enable();

    loop {
        // Wait for rising edge on STEP pin
        step_pin.wait_for_rising_edge().await;

        // Read direction
        let direction = dir_pin.is_high();
        controller.set_direction(direction);

        // Process step
        controller.step();
        controller.update_pwm(&mut pwm);
    }
}
```

---

### Priority 2: Motor Driver Safety Pins

**Severity:** HIGH - Safety Critical

**Status:** Not implemented

**Required Pins:**
| Pin | Function | Type | Notes |
|-----|----------|------|-------|
| PB4 | Motor Enable | GPIO Output | Active high to driver IC |
| PB5 | Fault Input | GPIO EXTI | Active low from driver |
| PC13 | Emergency Stop | GPIO Input | Optional external E-stop |

**Missing Functionality:**
1. Hardware enable/disable of motor driver
2. Fault detection from gate driver IC
3. Safe shutdown on fault conditions
4. Emergency stop input handling

**Impact:**
- Cannot safely disable motor in hardware fault conditions
- No protection against gate driver overcurrent/overtemperature
- Violates safe motor control practices
- Risk of hardware damage in fault conditions

**Recommended Implementation:**
```rust
pub struct MotorSafety {
    enable_pin: Output<'static, PB4>,
    fault_pin: ExtiInput<'static, PB5>,
}

impl MotorSafety {
    pub fn enable_motor(&mut self) {
        self.enable_pin.set_high();
    }

    pub fn disable_motor(&mut self) {
        self.enable_pin.set_low();
    }

    pub async fn wait_for_fault(&mut self) {
        self.fault_pin.wait_for_falling_edge().await;
        // Immediately disable PWM and log fault
    }
}
```

---

### Priority 3: DC Bus Voltage Monitoring

**Severity:** MEDIUM - Reliability

**Status:** Not implemented

**Required Pin:**
| Pin | Function | ADC Channel | Notes |
|-----|----------|-------------|-------|
| PC13 | Vbus Sense | ADC1_IN9 | Voltage divider 1:11 |

**Current ADC Usage:**
- PA0: Phase A current (ADC1_IN1) ✓
- PA1: Phase B current (ADC1_IN2) ✓
- Phase C calculated as `i_c = -i_a - i_b` ✓

**Missing:**
- DC bus voltage measurement
- Power supply health monitoring
- Undervoltage/overvoltage protection

**Impact:**
- Cannot detect power supply brown-out
- No warning before undervoltage conditions
- Cannot implement dynamic current limiting based on supply voltage

**Recommended Implementation:**
```rust
pub struct VoltageMonitor {
    vbus_raw: u16,
    voltage_v: f32,
}

impl VoltageMonitor {
    const DIVIDER_RATIO: f32 = 11.0;  // Hardware voltage divider
    const ADC_VREF: f32 = 3.3;
    const ADC_MAX: u16 = 4096;

    pub fn update(&mut self, adc_sample: u16) {
        self.vbus_raw = adc_sample;
        self.voltage_v = (adc_sample as f32 / Self::ADC_MAX as f32)
                        * Self::ADC_VREF
                        * Self::DIVIDER_RATIO;
    }

    pub fn is_undervoltage(&self) -> bool {
        self.voltage_v < 18.0  // Below 18V
    }

    pub fn is_overvoltage(&self) -> bool {
        self.voltage_v > 30.0  // Above 30V
    }
}
```

---

### Priority 4: Encoder Index Pulse

**Severity:** LOW - Feature Enhancement

**Status:** Not implemented

**Required Pin:**
| Pin | Function | Type | Notes |
|-----|----------|------|-------|
| PB7 | Encoder Index | GPIO EXTI | Once per revolution pulse |

**Current Encoder Interface:**
- SPI communication implemented (PA4/5/6/7)
- Supports TLE5012B and AS5047P
- Absolute position reading ✓

**Missing:**
- Index pulse for absolute calibration
- Reference position detection
- Multi-turn tracking

**Impact:**
- Cannot establish absolute zero position
- No reference for homing routines
- Limited to single-turn absolute positioning

---

### Priority 5: Status LEDs

**Severity:** LOW - User Experience

**Status:** Not implemented

**Recommended Pins:**
| Pin | Function | Type | Notes |
|-----|----------|------|-------|
| PB6 | Status LED | GPIO Output | Running/Fault/Idle indication |
| PB8 | CAN Activity LED | GPIO Output | Optional |

**Impact:**
- No visual feedback on board status
- Difficult to diagnose issues without debugger
- No indication of CAN communication activity

---

## 3. Incomplete Driver Implementations

### DMA Configuration Module

**File:** `src/firmware/drivers/dma.rs`

**Status:** Empty stub structure

**Current State:**
```rust
// Empty file - DMA is configured implicitly in other drivers
```

**Impact:**
- DMA channels configured implicitly in ADC and UART drivers
- No central DMA resource management
- Risk of DMA channel conflicts

**Recommendation:**
Either:
1. Remove the stub file if not needed
2. Implement central DMA configuration registry

---

### USB CDC Driver

**File:** `src/firmware/drivers/usb.rs:24`

**Status:** Stub only

**Evidence:**
```rust
// TODO: Full USB CDC implementation:
// - USB enumeration
// - CDC class descriptor
// - RX/TX endpoints
```

**Impact:**
- Listed feature is non-functional
- Cannot use USB for communication or firmware updates

**Recommendation:**
- Remove stub or mark as future work
- Document that USB is not supported in current version

---

## 4. Pin Configuration Source of Truth

### Where Pins Are Configured

**Current State:** Pin assignments scattered across multiple files

| Peripheral | Configuration File | Lines |
|------------|-------------------|-------|
| UART Debug | `src/firmware/system.rs` | 43-56 |
| CAN-FD | `src/firmware/system.rs` | 105-108 |
| PWM (3-phase) | `src/firmware/drivers/pwm.rs` | 29-34 |
| ADC (Current) | `src/firmware/drivers/adc.rs` | Implicit |
| SPI (Encoder) | `src/firmware/drivers/sensors.rs` | Implicit |

**Issues:**
1. No single source of truth for pin mappings
2. No schematic or pinout diagram
3. Pin conflicts not visible until runtime
4. Difficult to verify complete pin allocation

**Recommendations:**

**Create Hardware Pin Map Document:**
```markdown
# CLN17 v2.0 Pin Mapping

## STM32G431CB Pinout

| Pin | Primary Function | Alternate | Driver File | Notes |
|-----|------------------|-----------|-------------|-------|
| PA0 | ADC Phase A | - | adc.rs | ✓ |
| PA1 | ADC Phase B | - | adc.rs | ✓ |
| PA2 | **AVAILABLE** | USART2_TX | - | For UART relocation |
| PA3 | **AVAILABLE** | USART2_RX | - | For UART relocation |
| ... | ... | ... | ... | ... |
```

**Create Pin Configuration Module:**
```rust
// src/firmware/hardware/pins.rs
pub mod pins {
    // Document all pin assignments in one place
    pub const UART_TX: &str = "PA2";  // USART2
    pub const UART_RX: &str = "PA3";  // USART2
    pub const PWM_A_HIGH: &str = "PA8";   // TIM1_CH1
    pub const PWM_A_LOW: &str = "PA7";    // TIM1_CH1N
    // ... all pins documented here
}
```

---

## 5. Complete Pin Allocation Map

### Proposed Final Pin Assignment

| Pin | Function | Peripheral | Type | Status |
|-----|----------|------------|------|--------|
| **Power** |
| VSS | Ground | - | Power | - |
| VDD | 3.3V | - | Power | - |
| VBAT | Battery backup | - | Power | - |
| **Analog Inputs** |
| PA0 | Phase A Current | ADC1_IN1 | Analog | ✓ Configured |
| PA1 | Phase B Current | ADC1_IN2 | Analog | ✓ Configured |
| PC13 | DC Bus Voltage | ADC1_IN9 | Analog | ❌ Missing |
| **Three-Phase PWM (TIM1)** |
| PA8 | Phase A High-side | TIM1_CH1 | PWM Out | ✓ Configured |
| PA7 | Phase A Low-side | TIM1_CH1N | PWM Out | ✓ Configured |
| PA9 | Phase B High-side | TIM1_CH2 | PWM Out | ✓ Configured |
| PB0 | Phase B Low-side | TIM1_CH2N | PWM Out | ✓ Configured |
| PA10 | Phase C High-side | TIM1_CH3 | PWM Out | ✓ Configured |
| PB1 | Phase C Low-side | TIM1_CH3N | PWM Out | ✓ Configured |
| **Encoder Interface (SPI1)** |
| PA4 | Encoder CS | GPIO | Digital Out | ✓ Configured |
| PA5 | SPI Clock | SPI1_SCK | SPI | ✓ Configured |
| PA6 | SPI MISO | SPI1_MISO | SPI | ✓ Configured |
| PA7 | SPI MOSI | SPI1_MOSI | SPI | ⚠️ Conflict with PWM |
| PB7 | Encoder Index | EXTI | Digital In | ❌ Missing |
| **Communication** |
| PA2 | UART TX | USART2_TX | Serial | ❌ Proposed |
| PA3 | UART RX | USART2_RX | Serial | ❌ Proposed |
| PA11 | CAN RX | FDCAN1_RX | CAN-FD | ✓ Configured |
| PA12 | CAN TX | FDCAN1_TX | CAN-FD | ✓ Configured |
| **Step-Dir Interface** |
| PA15 | Step Input | EXTI | Digital In | ❌ Missing |
| PB3 | Direction Input | GPIO | Digital In | ❌ Missing |
| **Motor Safety** |
| PB4 | Motor Enable | GPIO | Digital Out | ❌ Missing |
| PB5 | Fault Input | EXTI | Digital In | ❌ Missing |
| **Status Indicators** |
| PB6 | Status LED | GPIO | Digital Out | ❌ Missing |
| **Debugging** |
| PA13 | SWDIO | SWD | Debug | Reserved |
| PA14 | SWCLK | SWD | Debug | Reserved |

### Pin Conflicts Identified

1. **PA7:** Used by both PWM (TIM1_CH1N) and SPI1_MOSI
   - **Resolution:** PA7 is primarily PWM, encoder SPI may need different pin

2. **PA9/PA10:** Used by both UART and PWM
   - **Resolution:** Move UART to PA2/PA3 (USART2)

---

## 6. TODO Items Audit

The codebase contains **62 TODO items**. Critical ones:

### Firmware TODOs (High Priority)

| File | Line | TODO | Priority |
|------|------|------|----------|
| `tasks/step_dir.rs` | 178 | Read step/dir inputs from GPIO | HIGH |
| `tasks/step_dir.rs` | 179 | Update PWM outputs based on position | HIGH |
| `system.rs` | 82 | Initialize other drivers (PWM, ADC, Encoder) | HIGH |
| `drivers/can.rs` | 142-148 | Actual CAN TX/RX via FDCAN HAL | MEDIUM |
| `drivers/usb.rs` | 24 | Full USB CDC implementation | LOW |
| `hardware/cordic.rs` | 18 | Configure CORDIC for sine/cosine | MEDIUM |
| `hardware/fmac.rs` | 42 | Configure FMAC for PI control | MEDIUM |

### Test TODOs (Medium Priority)

Multiple Robot Framework tests have TODO placeholders:
- GPIO step pulse injection (step_dir_control.robot:433)
- CAN frame capture (integration.robot:596)
- ADC injection for current sensing (foc_control.robot:546)

---

## 7. Recommendations Priority Matrix

### Immediate Action Required (Pre-Hardware Testing)

1. **Resolve PA9/PA10 UART/PWM conflict**
   - Move UART to USART2 (PA2/PA3)
   - Update system.rs initialization
   - Test UART + PWM simultaneously

2. **Implement motor enable/fault pins**
   - Add PB4 as enable output
   - Add PB5 as fault input with interrupt
   - Integrate with PWM disable logic

3. **Document complete pin mapping**
   - Create hardware/PINOUT.md
   - Add pin allocation to README
   - Generate pinout diagram

### High Priority (Feature Completion)

4. **Implement Step-Dir GPIO interface**
   - Add PA15/PB3 GPIO configuration
   - Implement EXTI for step pulses
   - Connect to StepDirController

5. **Add DC bus voltage monitoring**
   - Configure ADC1_IN9 on PC13
   - Add voltage divider circuit
   - Implement undervoltage protection

### Medium Priority (Quality & Safety)

6. **Add status LED indicators**
   - PB6 for general status
   - Implement fault/running/idle states

7. **Implement encoder index pulse**
   - PB7 EXTI configuration
   - Homing routine integration

### Low Priority (Future Enhancements)

8. **USB CDC implementation**
   - Complete drivers/usb.rs
   - Add firmware update capability

9. **CORDIC/FMAC hardware acceleration**
   - Configure STM32G4 math accelerators
   - Optimize FOC calculations

---

## 8. Risk Assessment

### Critical Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Pin conflict causes motor malfunction | High | High | Resolve PA9/PA10 conflict before hardware test |
| No fault detection damages hardware | High | Medium | Implement motor enable/fault pins |
| Step-Dir advertised but non-functional | Medium | High | Complete GPIO implementation or remove feature |

### Medium Risks

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| No voltage monitoring causes brown-out | Medium | Medium | Add Vbus ADC channel |
| Encoder SPI conflicts with PWM | Medium | Low | Verify PA7 usage on actual hardware |

---

## 9. Next Steps

### Week 1: Critical Fixes
- [ ] Move UART to USART2 (PA2/PA3)
- [ ] Test PWM + UART simultaneously in Renode
- [ ] Add motor enable/fault GPIO pins
- [ ] Create PINOUT.md documentation

### Week 2: Feature Completion
- [ ] Implement Step-Dir GPIO inputs
- [ ] Add DC bus voltage ADC channel
- [ ] Implement status LED control
- [ ] Update Renode platform with new pins

### Week 3: Testing & Validation
- [ ] Hardware-in-loop testing with actual CLN17 board
- [ ] Verify all pin assignments match schematic
- [ ] Test motor safety shutdown
- [ ] Validate Step-Dir pulse input at 50 kHz

### Week 4: Documentation & Release
- [ ] Complete hardware integration guide
- [ ] Update all code comments with pin references
- [ ] Create board bring-up checklist
- [ ] Final validation of all features

---

## 10. Conclusion

The CLN17 v2.0 firmware has a **solid foundation** with well-implemented FOC and Step-Dir control algorithms, comprehensive testing infrastructure, and clean Embassy async architecture. However, several **critical hardware integration gaps** must be addressed:

**Strengths:**
- ✓ Three-phase PWM with complementary outputs and dead-time
- ✓ Current sensing and ADC infrastructure
- ✓ CAN-FD communication with iRPC protocol
- ✓ Comprehensive Renode test suite
- ✓ Dual FOC/Step-Dir control modes

**Critical Gaps:**
- ❌ Pin conflicts blocking simultaneous UART + PWM use
- ❌ Missing motor safety pins (enable/fault)
- ❌ Step-Dir GPIO inputs not implemented
- ❌ No DC bus voltage monitoring
- ❌ No centralized pin documentation

**Recommendation:** Address the critical pin conflict and safety issues before any hardware deployment. The firmware is production-ready pending these hardware integration fixes.

---

**Document Version:** 1.0
**Last Updated:** 2025-11-10
**Author:** Firmware Analysis
**Status:** Ready for Review
