# Power Management & Monitoring Analysis - CLN17 V2.0

**Date:** 2025-11-10
**Status:** Gap Analysis & Improvement Recommendations
**Priority:** High - Critical for Motor Controller Safety & Efficiency

---

## Executive Summary

Current firmware has **basic** power monitoring (Vbus, current sensing) but lacks:
- ✅ Hardware support is excellent (STM32G431 + DRV8844)
- ⚠️ Software implementation is minimal (~20% complete)
- ❌ No active protection, thermal management, or efficiency optimization
- ❌ No predictive fault detection or power telemetry

**Estimated Improvement:** 80% more power-related features needed for production-grade motor controller.

---

## Current Implementation Assessment

### ✅ What We Have

#### 1. Voltage Monitoring (`src/firmware/drivers/adc.rs:49`)
```rust
// PA2 = ADC1_IN3 → Vbus voltage divider (1:15 ratio)
let vbus = p.PA2.degrade_adc();
```

**Features:**
- Reads supply voltage (8-48V range)
- Basic range checking:
  - `is_vbus_in_range()`: 8-48V nominal
  - `is_vbus_undervoltage()`: < 8V
  - `is_vbus_overvoltage()`: > 50V
- 15:1 voltage divider scaling

**Limitations:**
- ❌ No continuous monitoring task
- ❌ No brownout prediction
- ❌ No voltage sag detection
- ❌ No data logging

#### 2. Current Sensing (`src/firmware/drivers/adc.rs:45-46`)
```rust
// PA3 = ADC1_IN4  → Phase A current (AISEN)
// PB0 = ADC1_IN15 → Phase B current (BISEN)
```

**Features:**
- DRV8844 analog current outputs (0.2V/A)
- Calibration support (zero offset)
- Conversion to milliamps

**Limitations:**
- ❌ No active current limiting
- ❌ No overcurrent protection logic
- ❌ No RMS calculation
- ❌ No I²t thermal modeling
- ❌ Not integrated with PWM control

#### 3. Motor Driver Protection (`src/firmware/drivers/motor_driver.rs:83`)
```rust
pub fn is_fault(&self) -> bool {
    self.fault.is_low()  // PB1 = nFAULT from DRV8844
}
```

**Features:**
- Fault detection (overcurrent, overtemp, UVLO)
- Enable/disable control
- Reset capability
- Emergency stop

**Limitations:**
- ❌ No fault classification (can't tell OC from OT)
- ❌ No automatic recovery
- ❌ No fault statistics
- ❌ Reactive only (no prediction)

#### 4. Telemetry (`src/firmware/tasks/telemetry.rs`)
```rust
// TODO: Initialize USB CDC driver
```

**Status:** ❌ Not implemented (placeholder only)

---

## Hardware Capabilities (Unused)

### STM32G431CB Features Not Utilized

#### 1. **Internal Temperature Sensor** ❌
- **Capability:** ADC channel 16 (Vtemp)
- **Purpose:** MCU junction temperature monitoring
- **Use Case:** Thermal throttling, overtemp protection
- **Status:** Not implemented

#### 2. **Internal VREF Monitoring** ❌
- **Capability:** ADC channel 18 (Vrefint)
- **Purpose:** Accurate ADC calibration without external reference
- **Use Case:** Improve current/voltage measurement accuracy
- **Status:** Not implemented

#### 3. **Comparators (COMP1-3)** ❌
- **Capability:** 3 independent analog comparators
- **Purpose:** Hardware-level overcurrent trip (sub-microsecond response)
- **Use Case:** PWM emergency shutdown without CPU intervention
- **Status:** Not implemented

#### 4. **Low-Power Modes** ❌
- **Capability:** Sleep, Stop, Standby modes
- **Purpose:** Reduce idle power consumption
- **Use Case:** Step-Dir idle, sleep when motor disabled
- **Status:** Not implemented

#### 5. **DMA for ADC** ⚠️
- **Capability:** Continuous ADC sampling without CPU
- **Status:** Partially used (reads via DMA)
- **Missing:** Continuous circular buffer, triggered by PWM timer

#### 6. **ADC Watchdog** ❌
- **Capability:** Hardware threshold detection (high/low)
- **Purpose:** Instant interrupt on over/undervoltage
- **Use Case:** Faster than polling
- **Status:** Not implemented

### DRV8844 Features Not Utilized

#### 1. **Decay Mode Control** ❌
- **Pins:** Internal (default: smart tune)
- **Purpose:** Optimize efficiency and reduce ripple
- **Note:** Fixed in hardware, but monitoring would help
- **Status:** Not monitored

#### 2. **Current Regulation** ⚠️
- **Feature:** Adjustable current limit via VREF
- **CLN17 Hardware:** Fixed via resistor
- **Opportunity:** Software can respect this limit
- **Status:** Limit value not defined in code

---

## Critical Gaps & Risks

### 🔴 High Priority Issues

#### 1. **No Active Overcurrent Protection**
**Risk:** Motor damage, driver failure, fire hazard

**Current State:**
- DRV8844 has hardware protection (ILIMIT)
- Firmware reads current but doesn't act on it

**Needed:**
```rust
const MAX_PHASE_CURRENT_MA: i32 = 1750;  // 1.75A RMS → 2.47A peak
const CURRENT_LIMIT_MA: i32 = 2000;       // Software limit with margin

if abs(current_a_ma) > CURRENT_LIMIT_MA {
    pwm.reduce_duty();  // Active limiting
    // or
    motor_driver.disable();  // Emergency stop
}
```

#### 2. **No Thermal Management**
**Risk:** Silent MCU overheating, unreliable operation

**Needed:**
- Read STM32 internal temperature sensor
- Thermal throttling at 75°C
- Shutdown at 85°C
- Estimate motor temperature from I²R losses

**Example:**
```rust
let mcu_temp_c = read_internal_temp();
if mcu_temp_c > 75.0 {
    reduce_max_current(0.7);  // 70% current limit
    led.set_color(Yellow);     // Warning
}
if mcu_temp_c > 85.0 {
    motor_driver.emergency_stop();
    led.set_color(Red);
}
```

#### 3. **No Power Failure Detection**
**Risk:** Loss of position during brownout, uncontrolled deceleration

**Needed:**
- Vbus sag detection (< 10V)
- Rapid shutdown sequence
- Save encoder position to flash
- Signal fault on ERROR pin (PB3)

#### 4. **No RMS Current Calculation**
**Risk:** Exceeding DRV8844 thermal limits (1.75A RMS)

**Needed:**
```rust
// Running RMS calculation
let i_squared_sum = (current_a * current_a + current_b * current_b) / 2.0;
let i_rms = sqrt(i_squared_sum);  // Calculated over window

if i_rms > 1750.0 {  // 1.75A RMS limit
    trigger_current_limit();
}
```

### 🟡 Medium Priority Issues

#### 5. **No Energy/Efficiency Monitoring**
**Impact:** Cannot optimize performance, no diagnostics

**Needed:**
```rust
struct PowerMetrics {
    energy_mwh: u32,          // Total energy consumed
    avg_efficiency: f32,       // Mechanical / Electrical power
    power_factor: f32,         // Real / Apparent power
    time_active_ms: u32,       // Motor on time
}
```

#### 6. **No Regenerative Energy Detection**
**Impact:** Missing opportunity for energy recovery or braking control

**Needed:**
- Detect negative current (regeneration)
- Ensure Vbus doesn't exceed limit during regen
- Optional: brake resistor control

#### 7. **No Fault Statistics**
**Impact:** Difficult to diagnose intermittent issues

**Needed:**
```rust
struct FaultCounters {
    overcurrent_events: u16,
    overvoltage_events: u16,
    undervoltage_events: u16,
    overtemp_events: u16,
    driver_fault_events: u16,
    last_fault_timestamp: u32,
    fault_history: [FaultType; 16],  // Ring buffer
}
```

#### 8. **No Power Telemetry**
**Impact:** No visibility into power system status

**Needed:**
- Vbus, currents, power in telemetry stream
- Integration with `tasks/telemetry.rs`
- CAN, USB, and UART output

---

## Recommended Improvements

### Phase 1: Critical Safety (Must-Have)

#### A. Power Monitoring Task
**File:** `src/firmware/tasks/power_monitor.rs` (NEW)

```rust
#[embassy_executor::task]
pub async fn power_monitor(
    mut sensors: Sensors,
    mut motor_driver: MotorDriver,
    mut status_leds: StatusLeds,
) {
    let mut ticker = Ticker::every(Duration::from_millis(10));  // 100 Hz

    // State
    let mut vbus_samples = [0u32; 10];
    let mut sample_idx = 0;

    loop {
        ticker.next().await;

        // Read sensors
        let [ia_raw, ib_raw, vbus_raw] = sensors.read_all_raw().await;
        let vbus_mv = Sensors::raw_to_vbus_mv(vbus_raw);
        let ia_ma = Sensors::raw_to_milliamps(ia_raw, OFFSET_A);
        let ib_ma = Sensors::raw_to_milliamps(ib_raw, OFFSET_B);

        // === CRITICAL CHECKS ===

        // 1. Overvoltage protection
        if Sensors::is_vbus_overvoltage(vbus_mv) {
            motor_driver.emergency_stop();
            status_leds.set_color(Red);
            defmt::error!("OVERVOLTAGE: {} mV", vbus_mv);
        }

        // 2. Undervoltage protection
        if Sensors::is_vbus_undervoltage(vbus_mv) {
            motor_driver.emergency_stop();
            status_leds.set_color(Red);
            defmt::error!("UNDERVOLTAGE: {} mV", vbus_mv);
        }

        // 3. Overcurrent protection
        let i_total = ia_ma.abs() + ib_ma.abs();
        if i_total > PEAK_CURRENT_LIMIT_MA {
            motor_driver.emergency_stop();
            defmt::error!("OVERCURRENT: {} mA", i_total);
        }

        // 4. Driver fault check
        if motor_driver.is_fault() {
            motor_driver.disable();
            status_leds.set_color(Red);
            defmt::error!("DRV8844 FAULT detected");
            // Attempt recovery after delay
            Timer::after(Duration::from_millis(100)).await;
            motor_driver.reset();
        }

        // === TRENDING ANALYSIS ===

        // Detect voltage sag (brownout prediction)
        vbus_samples[sample_idx] = vbus_mv;
        sample_idx = (sample_idx + 1) % 10;
        let vbus_avg = vbus_samples.iter().sum::<u32>() / 10;

        if vbus_avg < 10000 && motor_driver.is_enabled() {
            defmt::warn!("Voltage sag detected: {} mV average", vbus_avg);
            // Could reduce current limit here
        }
    }
}
```

**Features:**
- ✅ 100 Hz monitoring rate
- ✅ Overvoltage/undervoltage protection
- ✅ Overcurrent emergency stop
- ✅ DRV8844 fault handling with auto-recovery
- ✅ Voltage sag detection (brownout)
- ✅ Status LED integration

**Estimated:** 4-6 hours implementation

---

#### B. MCU Temperature Monitoring
**File:** `src/firmware/drivers/adc.rs` (EXTEND)

```rust
impl Sensors {
    /// Read STM32 internal temperature sensor.
    ///
    /// Returns temperature in degrees Celsius.
    pub async fn read_mcu_temperature(&mut self) -> f32 {
        // ADC channel 16 = internal temperature sensor
        // Conversion: T(°C) = (V_SENSE - V_25°C) / Avg_Slope + 25
        // STM32G4: V_25°C ≈ 760 mV, Avg_Slope ≈ 2.5 mV/°C

        let temp_raw = self.adc.read_internal_temp().await;
        let temp_mv = (temp_raw as u32 * VREF_MV) / 4096;

        const V_25C_MV: f32 = 760.0;
        const AVG_SLOPE_MV_PER_C: f32 = 2.5;

        let temp_c = ((temp_mv as f32 - V_25C_MV) / AVG_SLOPE_MV_PER_C) + 25.0;
        temp_c
    }

    /// Check if MCU temperature is safe.
    pub fn is_mcu_temp_safe(temp_c: f32) -> bool {
        temp_c < 85.0  // Conservative limit
    }

    /// Get thermal throttle factor (0.0 to 1.0).
    ///
    /// Returns:
    /// - 1.0 if temp < 70°C (full power)
    /// - 0.7 if temp 70-80°C (reduced power)
    /// - 0.5 if temp 80-85°C (heavily reduced)
    /// - 0.0 if temp > 85°C (shutdown)
    pub fn get_thermal_throttle(temp_c: f32) -> f32 {
        if temp_c < 70.0 {
            1.0
        } else if temp_c < 80.0 {
            0.7
        } else if temp_c < 85.0 {
            0.5
        } else {
            0.0  // Emergency shutdown
        }
    }
}
```

**Add to power_monitor task:**
```rust
// Read MCU temperature every 1 second
if tick_counter % 100 == 0 {
    let mcu_temp = sensors.read_mcu_temperature().await;
    let throttle = Sensors::get_thermal_throttle(mcu_temp);

    // Apply thermal throttling
    set_max_current_limit((1750.0 * throttle) as i32);

    if throttle < 1.0 {
        defmt::warn!("Thermal throttle: {}%, temp: {:.1}°C",
                     throttle * 100.0, mcu_temp);
        status_leds.set_color(Yellow);
    }

    if throttle == 0.0 {
        motor_driver.emergency_stop();
        defmt::error!("MCU OVERTEMP: {:.1}°C - SHUTDOWN", mcu_temp);
    }
}
```

**Estimated:** 2-3 hours implementation

---

#### C. Hardware Comparator for Overcurrent
**File:** `src/firmware/drivers/comparator.rs` (NEW)

**Purpose:** Sub-microsecond hardware protection (faster than software polling)

```rust
/// Hardware comparator for emergency overcurrent shutdown.
///
/// Uses STM32G431 COMP2 to compare current sense voltage against threshold.
/// Triggers PWM break input for instant shutdown without CPU intervention.
pub struct OverCurrentComparator {
    comp: COMP2,
}

impl OverCurrentComparator {
    pub fn new(p: Peripherals, threshold_mv: u32) -> Self {
        // Configure COMP2:
        // - Positive input: PA3 (current sense A)
        // - Negative input: Internal VREF scaled to threshold
        // - Output: Connected to TIM2 break input

        // Calculate DAC value for threshold
        // If threshold = 2A, and DRV8844 = 0.2V/A:
        // V_threshold = 2A * 0.2V/A + V_offset = 0.4V + 1.65V

        // Configure comparator (pseudo-code, actual embassy API may differ)
        let mut comp = Comp::new(p.COMP2);
        comp.set_plus_input(CompInput::PA3);
        comp.set_minus_input(CompInput::Dac1Ch1);  // Use DAC for threshold
        comp.set_output_to_tim2_break();
        comp.enable();

        Self { comp }
    }
}
```

**Benefits:**
- ⚡ <1 µs response time (vs ~100 µs for software)
- 🛡️ Protection even if CPU locked up
- 🔧 Complements software monitoring

**Estimated:** 6-8 hours implementation (complex)

---

### Phase 2: Diagnostics & Optimization (Should-Have)

#### D. RMS Current Calculation
**File:** `src/firmware/tasks/power_monitor.rs` (EXTEND)

```rust
struct RmsCalculator {
    i_sq_buffer: [f32; 100],  // 100 samples @ 10 kHz = 10ms window
    index: usize,
}

impl RmsCalculator {
    fn update(&mut self, ia_ma: i32, ib_ma: i32) -> f32 {
        // I²t calculation
        let i_sq = ((ia_ma * ia_ma + ib_ma * ib_ma) / 2) as f32;
        self.i_sq_buffer[self.index] = i_sq;
        self.index = (self.index + 1) % 100;

        // Calculate RMS
        let sum: f32 = self.i_sq_buffer.iter().sum();
        let mean = sum / 100.0;
        mean.sqrt()
    }
}
```

**Use Case:**
```rust
let i_rms = rms_calc.update(ia_ma, ib_ma);
if i_rms > 1750.0 {  // DRV8844 RMS limit
    reduce_pwm_duty(0.9);  // Gradual current limiting
}
```

**Estimated:** 2 hours

---

#### E. Power & Efficiency Metrics
**File:** `src/firmware/tasks/power_monitor.rs` (EXTEND)

```rust
struct PowerMetrics {
    // Accumulated
    energy_mwh: u32,           // milliwatt-hours
    charge_mah: u32,           // milliamp-hours
    active_time_ms: u32,

    // Instantaneous
    power_mw: u32,             // Electrical power in
    mechanical_power_est_mw: u32,  // Estimated from torque × speed
    efficiency: f32,            // 0.0 to 1.0

    // Faults
    fault_counters: FaultCounters,
}

impl PowerMetrics {
    fn update(&mut self, vbus_mv: u32, ia_ma: i32, ib_ma: i32, dt_ms: u32) {
        // Electrical power: P = V × I
        let i_total_ma = ia_ma.abs() + ib_ma.abs();
        self.power_mw = (vbus_mv * i_total_ma as u32) / 1000;

        // Accumulate energy: E += P × Δt
        self.energy_mwh += (self.power_mw * dt_ms) / 3600000;  // mWh
        self.charge_mah += (i_total_ma as u32 * dt_ms) / 3600000;  // mAh

        // Mechanical power estimation (requires velocity and torque)
        // self.mechanical_power_est_mw = torque_mNm * velocity_rpm / 9550;

        // Efficiency (if mechanical power known)
        // self.efficiency = mechanical / electrical;
    }
}
```

**Estimated:** 3-4 hours

---

#### F. Enhanced Telemetry Integration
**File:** `src/firmware/tasks/telemetry.rs` (REWRITE)

```rust
#[embassy_executor::task]
pub async fn power_telemetry(
    power_metrics: &'static Mutex<PowerMetrics>,
) {
    let mut ticker = Ticker::every(Duration::from_millis(100));  // 10 Hz

    loop {
        ticker.next().await;

        let metrics = power_metrics.lock().await;

        // JSON-like telemetry format
        defmt::info!(
            "PWR: {{vbus:{},ia:{},ib:{},pwr:{},temp:{},flt:{}}}",
            metrics.vbus_mv,
            metrics.ia_ma,
            metrics.ib_ma,
            metrics.power_mw,
            metrics.mcu_temp_c,
            metrics.fault_counters.total()
        );

        // TODO: Send over CAN, USB, or UART
    }
}
```

**Features:**
- Real-time power data streaming
- Integration with existing telemetry
- CAN-FD, USB CDC, UART output
- Structured format for logging

**Estimated:** 2-3 hours

---

### Phase 3: Advanced Features (Nice-to-Have)

#### G. Predictive Fault Detection
- Trend analysis (voltage droop, temperature rise rate)
- Anomaly detection (unexpected current spikes)
- Early warning before hard faults

**Estimated:** 8-12 hours

#### H. Low-Power Modes
- Sleep when motor disabled for >10 seconds
- Wake on CAN message or Step pulse
- Standby mode for long-term idle

**Estimated:** 6-8 hours

#### I. Regenerative Braking Management
- Detect negative current flow
- Brake resistor control (if hardware present)
- Vbus clamp to prevent overvoltage

**Estimated:** 4-6 hours

---

## Implementation Priority Matrix

| Feature | Priority | Safety Impact | Effort | Value |
|---------|----------|---------------|--------|-------|
| Power Monitoring Task | 🔴 Critical | High | 6h | Very High |
| MCU Temp Monitoring | 🔴 Critical | High | 3h | High |
| Overcurrent Protection | 🔴 Critical | Very High | 4h | Very High |
| Hardware Comparator OC | 🟡 High | Very High | 8h | High |
| RMS Current Calc | 🟡 High | Medium | 2h | Medium |
| Power Metrics | 🟢 Medium | Low | 4h | Medium |
| Enhanced Telemetry | 🟢 Medium | Low | 3h | High |
| Predictive Faults | 🔵 Low | Medium | 12h | Medium |
| Low-Power Modes | 🔵 Low | None | 8h | Low |
| Regen Management | 🔵 Low | Medium | 6h | Medium |

**Total Estimated Effort:**
- **Phase 1 (Critical):** 18-20 hours
- **Phase 2 (Should-Have):** 7-10 hours
- **Phase 3 (Nice-to-Have):** 18-26 hours
- **Grand Total:** 43-56 hours

---

## Testing Requirements

### Power Protection Tests

1. **Overvoltage Test**
   - Apply 52V to Vbus
   - Verify emergency stop within 10ms
   - Check red LED activation

2. **Undervoltage Test**
   - Reduce Vbus to 7V
   - Verify emergency stop within 10ms

3. **Overcurrent Test**
   - Stall motor to exceed 2A
   - Verify current limiting or shutdown

4. **Thermal Test**
   - Heat MCU to 75°C, verify throttling
   - Heat to 85°C, verify shutdown

5. **Fault Recovery Test**
   - Trigger DRV8844 fault (short motor)
   - Verify reset and recovery

### Telemetry Tests

6. **Power Telemetry**
   - Verify Vbus, current, power reporting
   - Check update rate (10 Hz)

7. **Efficiency Calculation**
   - Compare electrical vs mechanical power
   - Verify reasonable efficiency (70-90%)

---

## Pin Utilization Summary

| Function | Pin | Current Use | New Use |
|----------|-----|-------------|---------|
| Vbus ADC | PA2 | ✅ Implemented | ✅ Enhanced monitoring |
| Current A | PA3 | ✅ Implemented | ✅ + RMS calc |
| Current B | PB0 | ✅ Implemented | ✅ + RMS calc |
| Motor Enable | PA4 | ✅ Implemented | ✅ + thermal throttle |
| Fault Input | PB1 | ✅ Implemented | ✅ + auto recovery |
| Temp Sensor | ADC16 | ❌ Unused | ✅ NEW: MCU temp |
| VREF Internal | ADC18 | ❌ Unused | ⚠️ Optional calibration |
| Comparator 2 | COMP2 | ❌ Unused | ⚠️ Optional HW OC |

**No new external pins required!** All improvements use existing or internal channels.

---

## Conclusion

The CLN17 V2.0 firmware has excellent **hardware foundation** for power management but **minimal software implementation**.

### Quick Wins (Phase 1):
1. ✅ Add power monitoring task (6h) → Immediate safety improvement
2. ✅ Add MCU temperature monitoring (3h) → Prevent thermal damage
3. ✅ Add software overcurrent protection (4h) → Motor/driver protection

**Total Phase 1:** ~13 hours for production-grade power protection.

### Recommended Next Steps:
1. Implement Phase 1 (critical safety)
2. Test thoroughly with hardware
3. Add Phase 2 (diagnostics) based on field needs
4. Consider Phase 3 (advanced) for premium features

**Risk if not implemented:**
- 🔥 Fire hazard from uncontrolled overcurrent
- 💥 Component damage from overvoltage/overtemp
- 📉 Poor reliability and user experience
- ⚖️ Potential liability issues

**Benefit if implemented:**
- ✅ Production-grade safety
- 📊 Excellent diagnostics and telemetry
- ⚡ Optimized efficiency
- 🏆 Competitive motor controller quality

---

**Document Version:** 1.0
**Author:** Power Management Analysis
**Status:** Recommendation - Awaiting Implementation Approval
**Next Action:** Review with stakeholders, prioritize phases
