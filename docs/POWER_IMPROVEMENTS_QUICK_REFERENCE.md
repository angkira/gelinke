# Power Management Improvements - Quick Reference

**TL;DR:** Current firmware has basic voltage/current sensing but lacks active protection, thermal management, and telemetry. ~20% complete → need 80% more work for production quality.

---

## Current State vs. Needed

### What We Have ✅

```
Hardware:               Software:
✅ PA2 → Vbus ADC       ✅ Read voltage
✅ PA3 → Current A      ✅ Read currents
✅ PB0 → Current B      ✅ Basic range checks
✅ PB1 → Fault detect   ✅ Enable/disable driver
                        ✅ Reset capability
```

### What's Missing ❌

```
Critical Gaps:
❌ No continuous power monitoring task
❌ No MCU temperature sensing (ADC16 unused)
❌ No active current limiting
❌ No thermal throttling
❌ No RMS current calculation
❌ No fault recovery logic
❌ No power telemetry
❌ No brownout prediction
❌ No efficiency metrics
❌ No hardware comparator protection
```

---

## Safety Risks Without Improvements

| Risk | Likelihood | Impact | Mitigation Needed |
|------|-----------|--------|-------------------|
| 🔥 Overcurrent damage | High | Severe | Active current limiting |
| 🌡️ MCU overheating | Medium | High | Temperature monitoring |
| ⚡ Overvoltage damage | Low | Severe | Enhanced Vbus monitoring |
| 📉 Brownout crash | Medium | Medium | Voltage sag detection |
| 💥 No fault recovery | High | Medium | Auto-reset logic |

**Conclusion:** Current implementation is **PROTOTYPE GRADE** - not suitable for production without Phase 1 improvements.

---

## Phase 1: Critical Safety (Must Do)

**Time:** 13-18 hours | **Value:** Very High | **Priority:** 🔴 Critical

### 1. Power Monitoring Task (6h)

**New File:** `src/firmware/tasks/power_monitor.rs`

**Features:**
- 100 Hz monitoring loop
- Overvoltage protection (>50V → emergency stop)
- Undervoltage protection (<8V → emergency stop)
- Overcurrent protection (>2A peak → emergency stop)
- Voltage sag detection (brownout prediction)
- DRV8844 fault handling with auto-recovery
- Status LED integration

**Key Code:**
```rust
#[embassy_executor::task]
pub async fn power_monitor(sensors, motor_driver, leds) {
    loop {
        // Read sensors @ 100 Hz
        let [ia, ib, vbus] = sensors.read_all_raw().await;

        // Critical checks
        if vbus > 50V { emergency_stop(); }
        if vbus < 8V  { emergency_stop(); }
        if ia + ib > 2A { emergency_stop(); }
        if driver.is_fault() { auto_recover(); }
    }
}
```

---

### 2. MCU Temperature Monitoring (3h)

**Modify:** `src/firmware/drivers/adc.rs`

**Features:**
- Read internal temp sensor (ADC channel 16)
- Thermal throttling at 70°C → 70% current limit
- Thermal throttling at 80°C → 50% current limit
- Emergency shutdown at 85°C

**Key Code:**
```rust
// Add to Sensors struct
pub async fn read_mcu_temperature(&mut self) -> f32 {
    let temp_raw = self.adc.read_internal_temp().await;
    convert_to_celsius(temp_raw)  // ~30-40°C typical
}

// Thermal management
let throttle = match mcu_temp {
    t if t < 70.0 => 1.0,   // Full power
    t if t < 80.0 => 0.7,   // Reduce to 70%
    t if t < 85.0 => 0.5,   // Reduce to 50%
    _ => 0.0,               // Emergency shutdown
};
```

**No new pins needed** - uses internal ADC channel!

---

### 3. Active Overcurrent Protection (4h)

**Modify:** `src/firmware/tasks/power_monitor.rs`

**Features:**
- RMS current calculation (10ms window)
- Software current limit: 1.75A RMS (DRV8844 spec)
- Peak current limit: 2.5A (transient)
- Gradual current reduction (not instant shutdown)

**Key Code:**
```rust
struct RmsCalculator {
    i_sq_buffer: [f32; 100],  // 100 samples
}

let i_rms = rms_calc.update(ia, ib);
if i_rms > 1750 {  // 1.75A RMS
    pwm.reduce_duty(0.9);  // Gradual limiting
} else if i_peak > 2500 {  // 2.5A peak
    motor_driver.emergency_stop();
}
```

---

## Phase 2: Diagnostics (Should Do)

**Time:** 7-10 hours | **Value:** High | **Priority:** 🟡 Medium

### 4. Power Metrics & Telemetry (4h)

**Features:**
- Real-time power calculation (P = V × I)
- Energy accumulation (mWh, mAh)
- Efficiency estimation
- Fault counters and history
- 10 Hz telemetry stream

**Output Example:**
```json
{
  "vbus_mv": 24000,
  "ia_ma": 850,
  "ib_ma": 820,
  "power_mw": 40000,
  "i_rms_ma": 1180,
  "mcu_temp_c": 42.5,
  "energy_mwh": 1250,
  "faults": {"oc": 0, "ov": 0, "uv": 1, "ot": 0}
}
```

### 5. Enhanced Telemetry Integration (3h)

**Modify:** `src/firmware/tasks/telemetry.rs`

**Features:**
- Power data in telemetry stream
- CAN, USB, UART output
- Configurable update rate
- Low overhead

---

## Phase 3: Advanced (Nice to Have)

**Time:** 18-26 hours | **Value:** Medium | **Priority:** 🔵 Low

### 6. Hardware Comparator Overcurrent (8h)
- Sub-microsecond protection
- Independent of CPU
- Emergency PWM shutdown

### 7. Predictive Fault Detection (12h)
- Trend analysis
- Early warnings
- Anomaly detection

### 8. Low-Power Modes (8h)
- Sleep when idle
- Wake on CAN/Step
- Extended battery life

---

## Implementation Checklist

### Phase 1 (Critical - Start Here)

- [ ] Create `src/firmware/tasks/power_monitor.rs`
  - [ ] 100 Hz monitoring loop
  - [ ] Overvoltage/undervoltage protection
  - [ ] Overcurrent protection
  - [ ] Fault auto-recovery
  - [ ] Status LED integration

- [ ] Extend `src/firmware/drivers/adc.rs`
  - [ ] Add `read_mcu_temperature()` method
  - [ ] Add `get_thermal_throttle()` function
  - [ ] Integrate with ADC channel 16

- [ ] Add RMS current calculation
  - [ ] Create `RmsCalculator` struct
  - [ ] 10ms sliding window
  - [ ] Integrate with current limiting

- [ ] Update `src/firmware/system.rs`
  - [ ] Spawn power_monitor task
  - [ ] Share sensors via Mutex/Channel
  - [ ] Wire up motor driver control

- [ ] Testing
  - [ ] Overvoltage test (52V)
  - [ ] Undervoltage test (7V)
  - [ ] Overcurrent test (stall motor)
  - [ ] Thermal test (heat MCU)
  - [ ] Fault recovery test

### Phase 2 (Diagnostics)

- [ ] Implement PowerMetrics struct
- [ ] Add power/energy calculations
- [ ] Add fault counters
- [ ] Rewrite telemetry.rs for power data
- [ ] Add CAN/USB/UART output

### Phase 3 (Advanced)

- [ ] Hardware comparator setup
- [ ] Predictive fault detection
- [ ] Low-power mode integration

---

## Resource Requirements

### No New Hardware Needed! ✅

All improvements use:
- ✅ Existing ADC channels (PA2, PA3, PB0)
- ✅ Existing motor driver pins (PA4, PB1, PB2)
- ✅ Internal ADC channels (temp sensor, VREF)
- ✅ Unused comparators (COMP2)

### Code Size Estimate

| Feature | Flash Usage | RAM Usage |
|---------|-------------|-----------|
| Power monitor task | ~2 KB | ~200 bytes |
| Temp monitoring | ~500 bytes | ~16 bytes |
| RMS calculator | ~800 bytes | ~400 bytes |
| Power metrics | ~1 KB | ~100 bytes |
| Telemetry | ~1.5 KB | ~200 bytes |
| **Total Phase 1-2** | **~6 KB** | **~1 KB** |

**Available:** 128 KB flash, 32 KB RAM → plenty of space!

---

## Expected Outcomes

### Before (Current State)

```
Safety:      ⭐⭐☆☆☆ (Basic fault detection)
Diagnostics: ⭐☆☆☆☆ (Minimal visibility)
Efficiency:  ⭐☆☆☆☆ (No optimization)
Reliability: ⭐⭐☆☆☆ (Reactive only)
```

### After Phase 1

```
Safety:      ⭐⭐⭐⭐⭐ (Multi-layer protection)
Diagnostics: ⭐⭐⭐☆☆ (Basic metrics)
Efficiency:  ⭐⭐☆☆☆ (Thermal throttling)
Reliability: ⭐⭐⭐⭐☆ (Auto-recovery)
```

### After Phase 2

```
Safety:      ⭐⭐⭐⭐⭐ (Comprehensive)
Diagnostics: ⭐⭐⭐⭐⭐ (Full telemetry)
Efficiency:  ⭐⭐⭐⭐☆ (Optimized)
Reliability: ⭐⭐⭐⭐⭐ (Production-grade)
```

---

## Quick Start: First Steps

1. **Review the detailed analysis:**
   - Read `docs/POWER_MANAGEMENT_ANALYSIS.md`
   - Understand hardware capabilities
   - Review safety risks

2. **Set up development environment:**
   - Ensure Rust toolchain ready
   - Test current ADC functionality
   - Verify motor driver pins working

3. **Start with power monitor task:**
   - Create `src/firmware/tasks/power_monitor.rs`
   - Copy template from analysis doc
   - Integrate with existing `system.rs`

4. **Test incrementally:**
   - Start with voltage monitoring only
   - Add current monitoring
   - Add fault handling
   - Add thermal monitoring

5. **Validate on hardware:**
   - Use adjustable power supply for Vbus tests
   - Use motor stall for overcurrent tests
   - Use heat gun for thermal tests (carefully!)

---

## Questions?

- **Q: Do I need new hardware?**
  - A: No! All improvements use existing pins + internal ADC channels.

- **Q: How much flash/RAM will this use?**
  - A: Phase 1-2: ~6 KB flash, ~1 KB RAM (plenty available).

- **Q: What's the minimum viable implementation?**
  - A: Just the power monitor task (6h) gives 80% of the safety benefit.

- **Q: Can I skip Phase 1 and go straight to Phase 2?**
  - A: **NO!** Phase 1 is critical safety. Phase 2 is diagnostics.

- **Q: How do I test without hardware?**
  - A: Use Renode emulation, but hardware validation is essential.

---

**Next Step:** Review detailed analysis → Implement Phase 1 → Test → Deploy

**See Also:**
- `docs/POWER_MANAGEMENT_ANALYSIS.md` - Full technical analysis
- `docs/CLN17_V2_HARDWARE_PINOUT.md` - Hardware reference
- `src/firmware/drivers/adc.rs` - Current ADC implementation
- `src/firmware/drivers/motor_driver.rs` - Motor control
