# CLN17 V2.0 Hardware Pinout - Official Specification

**Board:** CLN17 V2.0 Closed-Loop NEMA17 Driver
**MCU:** STM32G431CB (Cortex-M4F @ 170 MHz)
**Motor Driver:** DRV8844 (8-48V, 1.75A RMS)
**Encoder:** TLE5012B (15-bit magnetic, SPI)
**Source:** https://github.com/creapunk/TunePulse/blob/main/src/target/cln17_v2_0/target.h
**Date:** 2025-11-10

---

## Complete STM32G431CB Pin Mapping

This is the **authoritative** pin configuration from the official TunePulse firmware for CLN17 V2.0 hardware.

### Pin Allocation Table

| Pin   | Primary Function | Peripheral | Type | Connector | Notes |
|-------|------------------|------------|------|-----------|-------|
| **Port A** |
| PA0   | Motor Phase A1 | TIM2_CH1 | PWM Out | - | DRV8844 AIN1 |
| PA1   | Motor Phase A2 | TIM2_CH2 | PWM Out | - | DRV8844 AIN2 |
| PA2   | Supply Voltage Sense | ADC1_IN3 | Analog In | - | Vbus monitoring |
| PA3   | Current Sense A | ADC1_IN4 | Analog In | - | DRV8844 AISEN |
| PA4   | Motor Driver Enable | GPIO | Digital Out | - | DRV8844 nSLEEP |
| PA5   | Encoder SPI Clock | SPI1_SCK | SPI | - | TLE5012B SCLK |
| PA6   | Encoder SPI MISO | SPI1_MISO | SPI | - | TLE5012B DATA |
| PA7   | Encoder SPI MOSI | SPI1_MOSI | SPI | - | TLE5012B (unused) |
| PA8   | CTRL Enable | GPIO | Digital In | CTRL | Step-Dir enable input |
| PA9   | CAN Shutdown | GPIO | Digital Out | - | CAN transceiver SHDN (active low) |
| PA10  | CAN I/O Control | GPIO | Digital Out | - | CAN transceiver S |
| PA11  | USB D- | USB_DM | USB | USB | USB data negative |
| PA12  | USB D+ | USB_DP | USB | USB | USB data positive |
| PA13  | SWDIO | SWD | Debug | SWD | Debug interface |
| PA14  | SWCLK | SWD | Debug | SWD | Debug clock |
| PA15  | User Switch 1 | GPIO | Digital In | - | Pullup, active low |
| **Port B** |
| PB0   | Current Sense B | ADC1_IN15 | Analog In | - | DRV8844 BISEN |
| PB1   | Motor Driver Error | GPIO | Digital In | - | DRV8844 nFAULT |
| PB2   | Motor Driver Reset | GPIO | Digital Out | - | DRV8844 nRESET |
| PB3   | CTRL Error Output | GPIO | Digital Out | CTRL | Error signal output |
| PB4   | CTRL Direction | GPIO | Digital In | CTRL | Step-Dir direction input |
| PB5   | CTRL Step | GPIO | Digital In | CTRL | Step-Dir step input |
| PB6   | - | - | - | - | **Not used** |
| PB7   | User Switch 2 | GPIO | Digital In | - | Pullup, active low |
| PB8   | CAN RX | FDCAN1_RX | CAN-FD | CAN | CAN receive |
| PB9   | CAN TX | FDCAN1_TX | CAN-FD | CAN | CAN transmit |
| PB10  | Motor Phase B2 | TIM2_CH3 | PWM Out | - | DRV8844 BIN2 |
| PB11  | Motor Phase B1 | TIM2_CH4 | PWM Out | - | DRV8844 BIN1 |
| PB12  | CTRL 5V Output Control | GPIO | Digital Out | CTRL | Open-drain 5V enable |
| PB13  | Status LED Red | GPIO | Digital Out | - | Active low |
| PB14  | Status LED Green | GPIO | Digital Out | - | Active low |
| PB15  | Status LED Blue | GPIO | Digital Out | - | Active low |
| **Port C** |
| PC4   | Encoder SPI CS | GPIO | Digital Out | - | TLE5012B chip select |
| PC10  | Expansion UART TX | USART3_TX | Serial | EXP | Expansion connector |
| PC11  | Expansion UART RX | USART3_RX | Serial | EXP | Expansion connector |
| PC13  | - | - | - | - | **Not used** |
| PC14  | OSC32_IN | RCC_OSC32_IN | Clock | - | 32.768 kHz crystal (optional) |
| PC15  | OSC32_OUT | RCC_OSC32_OUT | Clock | - | 32.768 kHz crystal (optional) |

---

## Peripheral Assignments

### Motor Control (DRV8844)

**Timer:** TIM2 (4-channel PWM)

| Function | Pin | Timer Channel | Notes |
|----------|-----|---------------|-------|
| Phase A High (AIN1) | PA0 | TIM2_CH1 | H-bridge control |
| Phase A Low (AIN2) | PA1 | TIM2_CH2 | H-bridge control |
| Phase B High (BIN1) | PB11 | TIM2_CH4 | H-bridge control |
| Phase B Low (BIN2) | PB10 | TIM2_CH3 | H-bridge control |
| Enable (nSLEEP) | PA4 | GPIO | Active high |
| Reset (nRESET) | PB2 | GPIO | Active low |
| Fault (nFAULT) | PB1 | GPIO Input | Active low, read status |

**PWM Configuration:**
- Frequency: Typically 20-40 kHz
- Mode: Independent channel control (not complementary)
- Dead-time: Not needed (DRV8844 has internal protection)

---

### Current Sensing (ADC1)

| Function | Pin | ADC Channel | Notes |
|----------|-----|-------------|-------|
| Phase A Current | PA3 | ADC1_IN4 | DRV8844 AISEN output |
| Phase B Current | PB0 | ADC1_IN15 | DRV8844 BISEN output |
| Supply Voltage | PA2 | ADC1_IN3 | Vbus voltage divider |

**ADC Configuration:**
- Resolution: 12-bit (4096 counts)
- Vref: 3.3V
- DMA: Required for continuous sampling
- Trigger: TIM2 update (synchronize with PWM)

---

### Encoder Interface (SPI1)

| Function | Pin | SPI Signal | Notes |
|----------|-----|------------|-------|
| Clock | PA5 | SPI1_SCK | Up to 8 MHz |
| MISO | PA6 | SPI1_MISO | Data from TLE5012B |
| MOSI | PA7 | SPI1_MOSI | Not used by TLE5012B |
| Chip Select | PC4 | GPIO | Active low |

**Encoder:** TLE5012B
- Resolution: 15-bit (32768 counts/rev)
- Protocol: SPI read-only
- Update rate: Up to 1 kHz recommended

---

### Step-Dir Interface (CTRL Connector)

**Connector:** XH2.5-6P

| Pin# | Function | MCU Pin | Type | Notes |
|------|----------|---------|------|-------|
| 1 | GND | - | Power | Ground |
| 2 | 5V | PB12 | Power | Controlled by PB12 (open-drain) |
| 3 | ENABLE | PA8 | Input | Active high, pullup |
| 4 | STEP | PB5 | Input | Rising edge triggered |
| 5 | DIRECTION | PB4 | Input | High = forward, Low = reverse |
| 6 | ERROR | PB3 | Output | Active high on fault |

**Configuration:**
- STEP: EXTI interrupt on rising edge
- DIR: GPIO input, read on each step
- ENABLE: GPIO input, enable motor when high
- ERROR: GPIO output, set high on faults

---

### CAN-FD Communication

| Function | Pin | Peripheral | Notes |
|----------|-----|------------|-------|
| CAN RX | PB8 | FDCAN1_RX | To CAN transceiver RXD |
| CAN TX | PB9 | FDCAN1_TX | To CAN transceiver TXD |
| CAN Shutdown | PA9 | GPIO | Transceiver SHDN (active low) |
| CAN I/O Control | PA10 | GPIO | Transceiver S pin |

**Configuration:**
- Protocol: CAN-FD
- Nominal Bitrate: 1 Mbps
- Data Bitrate: 5 Mbps
- Transceiver: Likely TJA1051 or similar

---

### USB Communication

| Function | Pin | USB Signal | Notes |
|----------|-----|------------|-------|
| USB D+ | PA12 | USB_DP | Data positive |
| USB D- | PA11 | USB_DM | Data negative |

**Configuration:**
- USB 2.0 Full Speed (12 Mbps)
- Device mode (CDC-ACM)
- Used for configuration and firmware updates

---

### Expansion UART (USART3)

| Function | Pin | USART Signal | Notes |
|----------|-----|--------------|-------|
| TX | PC10 | USART3_TX | Transmit data |
| RX | PC11 | USART3_RX | Receive data |

**Purpose:** External communication, telemetry, or additional protocols

---

### Status LEDs

| Color | Pin | Active Level | Notes |
|-------|-----|--------------|-------|
| Red | PB13 | Low | Error/fault indication |
| Green | PB14 | Low | Running/active indication |
| Blue | PB15 | Low | User-defined |

**LED Control:** Active low (set pin LOW to turn LED ON)

---

### User Switches

| Switch | Pin | Active Level | Pull | Notes |
|--------|-----|--------------|------|-------|
| Switch 1 | PA15 | Low | Pullup | User button 1 |
| Switch 2 | PB7 | Low | Pullup | User button 2 |

---

## Power and System

### Power Input
- **Connector:** XH2.5-2P (PWR)
- **Voltage Range:** 8-48V DC
- **Current:** Up to 1.75A RMS to motor

### Internal Power Rails
- **5V Rail:** Buck converter from Vin
- **3.3V Rail:** LDO from 5V (MCU power)
- **5V Output:** Controlled by PB12 (for external devices)

### Clock Configuration
- **HSE:** 10 ppm external crystal (frequency not specified in pin file)
- **LSE:** Optional 32.768 kHz on PC14/PC15
- **PLL:** 170 MHz system clock

---

## Important Design Notes

### 1. PWM is NOT Complementary
Unlike typical FOC drivers, CLN17 uses **independent PWM channels** on TIM2, not complementary outputs. The DRV8844 has:
- 4 independent inputs (AIN1, AIN2, BIN1, BIN2)
- Internal H-bridge control
- No need for dead-time insertion in firmware

### 2. Current Sensing
- DRV8844 outputs analog current sense signals (AISEN, BISEN)
- These are **not** shunt resistor measurements
- ADC values proportional to motor current

### 3. Encoder Position
- TLE5012B provides absolute position within one revolution
- No index pulse available
- Multi-turn tracking requires software integration

### 4. Step-Dir Compatibility
- Hardware supports legacy Step/Dir interfaces
- STEP input should use EXTI for fast response
- Maximum step frequency: ~50 kHz (depends on firmware)

### 5. CAN and USB Conflict
- PA11/PA12 shared between USB and CAN control signals
- CAN uses PB8/PB9 (no conflict)
- USB and CAN can coexist

---

## Comparison with Current Firmware

### ❌ **Critical Differences Found:**

| Peripheral | Current Firmware | Actual Hardware | Status |
|------------|------------------|-----------------|--------|
| **PWM Timer** | TIM1 (complementary) | TIM2 (independent) | ❌ **WRONG** |
| **PWM Pins** | PA8/PA7, PA9/PB0, PA10/PB1 | PA0, PA1, PB10, PB11 | ❌ **WRONG** |
| **PWM Mode** | Complementary | Independent | ❌ **WRONG** |
| **Current A** | PA0 | PA3 | ❌ **WRONG** |
| **Current B** | PA1 | PB0 | ✓ Correct |
| **Vbus ADC** | Not implemented | PA2 (ADC1_IN3) | ❌ **MISSING** |
| **Encoder CS** | PA4 | PC4 | ❌ **WRONG** |
| **Encoder SPI** | PA5/PA6/PA7 | PA5/PA6/PA7 | ✓ Correct |
| **CAN RX** | PA11 | PB8 | ❌ **WRONG** |
| **CAN TX** | PA12 | PB9 | ❌ **WRONG** |
| **UART** | USART1 PA9/PA10 | USART3 PC10/PC11 | ❌ **WRONG** |
| **Step Input** | Not implemented | PB5 | ❌ **MISSING** |
| **Dir Input** | Not implemented | PB4 | ❌ **MISSING** |
| **Enable Input** | Not implemented | PA8 | ❌ **MISSING** |
| **Motor Enable** | Not implemented | PA4 | ❌ **MISSING** |
| **Fault Input** | Not implemented | PB1 | ❌ **MISSING** |
| **Status LEDs** | Not implemented | PB13/PB14/PB15 | ❌ **MISSING** |

---

## Immediate Actions Required

### 1. **Complete PWM Rewrite** (CRITICAL)
- Change from TIM1 complementary to TIM2 independent
- Remap all PWM pins: PA0, PA1, PB10, PB11
- Remove dead-time configuration (not needed)
- Implement 4-channel independent control

### 2. **ADC Reconfiguration** (CRITICAL)
- Move Current A from PA0 → PA3
- Current B already correct (PB0)
- Add Vbus monitoring on PA2
- Reconfigure ADC channels and DMA

### 3. **CAN Pin Correction** (CRITICAL)
- Move from PA11/PA12 → PB8/PB9
- Add CAN transceiver control (PA9, PA10)
- This frees PA11/PA12 for USB

### 4. **UART Relocation** (HIGH)
- Move from USART1 (PA9/PA10) → USART3 (PC10/PC11)
- Update DMA channel assignments

### 5. **Step-Dir GPIO Implementation** (HIGH)
- Add EXTI on PB5 (STEP)
- Add GPIO read on PB4 (DIR)
- Add GPIO read on PA8 (ENABLE)
- Add GPIO output on PB3 (ERROR)

### 6. **Motor Safety Pins** (HIGH)
- Add GPIO output PA4 (motor enable)
- Add GPIO input PB1 (fault detection)
- Add GPIO output PB2 (driver reset)

### 7. **Status LEDs** (MEDIUM)
- Add GPIO outputs PB13/PB14/PB15
- Implement active-low control

---

## Verification Checklist

Before hardware testing:

- [ ] All PWM pins match hardware (PA0, PA1, PB10, PB11)
- [ ] TIM2 configured for independent channels
- [ ] Current sense ADC on PA3 and PB0
- [ ] Vbus monitoring on PA2
- [ ] Encoder CS on PC4 (not PA4)
- [ ] CAN on PB8/PB9 (not PA11/PA12)
- [ ] UART on PC10/PC11 (not PA9/PA10)
- [ ] Step-Dir GPIO on PB5/PB4/PA8
- [ ] Motor enable on PA4
- [ ] Fault detection on PB1
- [ ] Status LEDs on PB13/PB14/PB15
- [ ] No pin conflicts remaining

---

**Document Version:** 1.0
**Source:** Official TunePulse firmware target.h
**Status:** Authoritative Hardware Specification
**Last Updated:** 2025-11-10
