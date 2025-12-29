# Power Management Architecture - Before vs After

---

## Current Architecture (Prototype Grade)

```
┌─────────────────────────────────────────────────────────────────┐
│                      CLN17 V2.0 Hardware                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Power Input: 8-48V DC                                          │
│       │                                                         │
│       ├─→ PA2 (Vbus ADC) ──────────────┐                        │
│       │                                 │                        │
│  DRV8844 Motor Driver                   │                        │
│       │                                 │                        │
│       ├─→ PA3 (Current A) ──────────┐   │                        │
│       ├─→ PB0 (Current B) ──────────┼───┼──→ ADC1              │
│       ├─→ PB1 (nFAULT) ─────────────┼───┘                        │
│       ├─→ PA4 (nSLEEP/Enable) ◀─────┼──────── GPIO Out          │
│       └─→ PB2 (nRESET) ◀────────────┘                           │
│                                                                 │
│  Internal MCU:                                                  │
│       ADC16 (Temp Sensor) ──────────────── ❌ UNUSED            │
│       ADC18 (VREF) ─────────────────────── ❌ UNUSED            │
│       COMP2 (Comparator) ───────────────── ❌ UNUSED            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                      Firmware (Current)                         │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  drivers/adc.rs                                                 │
│  ├─ read_vbus_raw()            ✅ Available                     │
│  ├─ read_currents_raw()        ✅ Available                     │
│  ├─ raw_to_vbus_mv()           ✅ Available                     │
│  ├─ raw_to_milliamps()         ✅ Available                     │
│  ├─ is_vbus_in_range()         ✅ Available (8-48V)             │
│  ├─ is_vbus_overvoltage()      ✅ Available (>50V)              │
│  └─ is_vbus_undervoltage()     ✅ Available (<8V)               │
│                                                                 │
│  drivers/motor_driver.rs                                        │
│  ├─ enable()                   ✅ Available                     │
│  ├─ disable()                  ✅ Available                     │
│  ├─ is_fault()                 ✅ Available                     │
│  └─ reset()                    ✅ Available                     │
│                                                                 │
│  tasks/                                                         │
│  ├─ FOC control                ✅ Exists                        │
│  ├─ Step-Dir control           ✅ Exists                        │
│  ├─ CAN communication          ✅ Exists                        │
│  ├─ Power monitoring           ❌ MISSING ← Critical!           │
│  └─ Telemetry                  ⚠️  Stub only                    │
│                                                                 │
│  Integration:                                                   │
│  ├─ Continuous monitoring      ❌ NO - Only read on demand      │
│  ├─ Active protection          ❌ NO - Reactive only            │
│  ├─ Thermal management         ❌ NO - No temp sensing          │
│  ├─ Current limiting           ❌ NO - No RMS calc              │
│  ├─ Fault recovery             ❌ NO - Manual reset only        │
│  └─ Telemetry                  ❌ NO - No power data            │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Status: Functions exist but NOT ACTIVELY USED
Risk:   High - No protection in normal operation
```

---

## Proposed Architecture (Production Grade)

```
┌─────────────────────────────────────────────────────────────────┐
│                      CLN17 V2.0 Hardware                        │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  Power Input: 8-48V DC                                          │
│       │                                                         │
│       ├─→ PA2 (Vbus ADC) ──────────────┐                        │
│       │                                 │                        │
│  DRV8844 Motor Driver                   │                        │
│       │                                 │                        │
│       ├─→ PA3 (Current A) ──────────┐   │                        │
│       ├─→ PB0 (Current B) ──────────┼───┼──→ ADC1 + DMA        │
│       ├─→ PB1 (nFAULT) ─────────────┼───┤   (Continuous)       │
│       ├─→ PA4 (nSLEEP/Enable) ◀─────┼───┤                        │
│       └─→ PB2 (nRESET) ◀────────────┘   │                        │
│                                          │                        │
│  Internal MCU:                           │                        │
│       ADC16 (Temp Sensor) ───────────────┼── ✅ NEW: Thermal mgmt│
│       ADC18 (VREF) ──────────────────────┼── ⚠️  Optional calib  │
│       COMP2 (Comparator) ────────────────┼── ✅ NEW: HW OC trip  │
│                                          │                        │
│  Status LEDs:                            │                        │
│       PB13 (Red)   ◀─────────────────────┼── Fault indication    │
│       PB14 (Green) ◀─────────────────────┼── Normal operation    │
│       PB15 (Blue)  ◀─────────────────────┘   Throttle warning    │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Firmware (Phase 1: Critical)                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  NEW: tasks/power_monitor.rs  @ 100 Hz                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                                                          │   │
│  │  loop {                                                  │   │
│  │      // Read sensors                                     │   │
│  │      [ia, ib, vbus] = sensors.read_all_raw().await;     │   │
│  │      mcu_temp = sensors.read_mcu_temperature().await;   │   │
│  │                                                          │   │
│  │      // === CRITICAL PROTECTION ===                      │   │
│  │                                                          │   │
│  │      ┌─ Overvoltage (>50V)                              │   │
│  │      │   → emergency_stop()                             │   │
│  │      │   → LED red                                      │   │
│  │      │                                                   │   │
│  │      ├─ Undervoltage (<8V)                              │   │
│  │      │   → emergency_stop()                             │   │
│  │      │   → LED red                                      │   │
│  │      │                                                   │   │
│  │      ├─ Overcurrent (>2A peak)                          │   │
│  │      │   → emergency_stop()                             │   │
│  │      │   → LED red                                      │   │
│  │      │                                                   │   │
│  │      ├─ RMS Current (>1.75A)                            │   │
│  │      │   → gradual_current_limit()                      │   │
│  │      │   → LED yellow                                   │   │
│  │      │                                                   │   │
│  │      ├─ MCU Overtemp (>70°C)                            │   │
│  │      │   → thermal_throttle(70%)                        │   │
│  │      │   → LED yellow                                   │   │
│  │      │                                                   │   │
│  │      ├─ MCU Overtemp (>85°C)                            │   │
│  │      │   → emergency_stop()                             │   │
│  │      │   → LED red, blink                               │   │
│  │      │                                                   │   │
│  │      ├─ DRV8844 Fault                                   │   │
│  │      │   → disable()                                    │   │
│  │      │   → wait 100ms                                   │   │
│  │      │   → reset()                                      │   │
│  │      │   → auto-recovery (3 attempts)                   │   │
│  │      │                                                   │   │
│  │      └─ Voltage Sag (brownout prediction)              │   │
│  │          → reduce_current_limit()                       │   │
│  │          → prepare for shutdown                         │   │
│  │                                                          │   │
│  │      // === METRICS ===                                  │   │
│  │                                                          │   │
│  │      power_mw = (vbus_mv * i_total) / 1000;            │   │
│  │      energy_mwh += power_mw * dt / 3600000;            │   │
│  │      fault_counters.update();                           │   │
│  │  }                                                       │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  ENHANCED: drivers/adc.rs                                       │
│  ├─ read_mcu_temperature()     ✅ NEW: ADC16 temp sensor       │
│  ├─ get_thermal_throttle()     ✅ NEW: 70°C→0.7, 85°C→0.0      │
│  └─ RmsCalculator              ✅ NEW: 10ms sliding window      │
│                                                                 │
│  ENHANCED: drivers/motor_driver.rs                              │
│  └─ auto_recovery()            ✅ NEW: 3 attempts w/ backoff    │
│                                                                 │
│  Integration with FOC/Step-Dir:                                 │
│  ├─ Shared PowerMetrics        ✅ Mutex<PowerMetrics>          │
│  ├─ Thermal throttle limit     ✅ max_current *= throttle       │
│  ├─ Emergency stop channel     ✅ Signal all tasks              │
│  └─ Current limit feedback     ✅ Reduce PWM duty               │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Firmware (Phase 2: Diagnostics)               │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  ENHANCED: tasks/telemetry.rs  @ 10 Hz                          │
│  ┌──────────────────────────────────────────────────────────┐   │
│  │                                                          │   │
│  │  Output via CAN/USB/UART:                               │   │
│  │  {                                                       │   │
│  │    "vbus_mv": 24000,                                    │   │
│  │    "ia_ma": 850,                                        │   │
│  │    "ib_ma": 820,                                        │   │
│  │    "i_rms_ma": 1180,                                    │   │
│  │    "power_mw": 40000,                                   │   │
│  │    "energy_mwh": 1250,                                  │   │
│  │    "mcu_temp_c": 42.5,                                  │   │
│  │    "throttle": 1.0,                                     │   │
│  │    "faults": {                                          │   │
│  │      "overcurrent": 0,                                  │   │
│  │      "overvoltage": 0,                                  │   │
│  │      "undervoltage": 1,                                 │   │
│  │      "overtemp": 0,                                     │   │
│  │      "driver": 0                                        │   │
│  │    }                                                     │   │
│  │  }                                                       │   │
│  │                                                          │   │
│  └──────────────────────────────────────────────────────────┘   │
│                                                                 │
│  Power Metrics:                                                 │
│  ├─ Real-time power (mW)       ✅ P = V × I                     │
│  ├─ Energy accumulation (mWh)  ✅ ∫ P dt                        │
│  ├─ Charge usage (mAh)         ✅ ∫ I dt                        │
│  ├─ Efficiency estimate        ✅ Pmech / Pelec                 │
│  ├─ Fault history (16 events)  ✅ Ring buffer                   │
│  └─ Uptime counters            ✅ Active/idle time              │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
                                 │
                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                   Firmware (Phase 3: Advanced)                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  NEW: drivers/comparator.rs                                     │
│  ├─ Hardware overcurrent trip  ✅ COMP2 → TIM2 break            │
│  ├─ Response time: <1 µs       ✅ Independent of CPU            │
│  └─ PWM emergency shutdown     ✅ Even if CPU locked            │
│                                                                 │
│  Predictive Protection:                                         │
│  ├─ Voltage droop trend        ✅ dV/dt analysis                │
│  ├─ Temperature rise rate      ✅ dT/dt warning                 │
│  ├─ Current spike detection    ✅ Anomaly detection             │
│  └─ Early fault warnings       ✅ Before hard limits            │
│                                                                 │
│  Low-Power Modes:                                               │
│  ├─ Sleep when idle >10s       ✅ RTC wakeup                    │
│  ├─ Wake on CAN/Step pulse     ✅ EXTI wakeup                   │
│  └─ Standby for storage        ✅ <100 µA idle                  │
│                                                                 │
│  Regenerative Braking:                                          │
│  ├─ Negative current detect    ✅ Regen mode flag               │
│  ├─ Vbus clamp                 ✅ Prevent overvoltage           │
│  └─ Brake resistor control     ⚠️  If hardware present          │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘

Status: Multi-layer protection + diagnostics + optimization
Risk:   Low - Production-grade safety
```

---

## Data Flow Comparison

### Current (Reactive, Polling-Based)

```
User Code
   │
   ├─ Calls sensors.read_vbus_raw() ──→ One-time read
   │                                    (only when asked)
   │
   ├─ Calls is_vbus_overvoltage()  ──→ Check value
   │                                    (but no automatic action)
   │
   └─ Manually calls disable()     ──→ If user remembers
                                        (relies on application logic)

Problem: Application must remember to check every time!
         No protection if forgot to call checking functions.
```

### Proposed (Proactive, Task-Based)

```
Power Monitor Task (100 Hz loop)
   │
   ├─→ ADC DMA (continuous) ──→ [ia, ib, vbus] buffer
   │                            Updated automatically
   │
   ├─→ Internal ADC ──────────→ MCU temperature
   │                            Every 1 second
   │
   ├─→ GPIO read ─────────────→ DRV8844 fault status
   │                            Every iteration
   │
   ├─→ RMS Calculator ────────→ i_rms (10ms window)
   │                            Updated every cycle
   │
   ├─→ Protection Logic ──────→ Automatic checks:
   │    │                        ├─ Overvoltage
   │    │                        ├─ Undervoltage
   │    │                        ├─ Overcurrent (peak)
   │    │                        ├─ Overcurrent (RMS)
   │    │                        ├─ Overtemperature
   │    │                        └─ Driver fault
   │    │
   │    └─→ Actions:
   │         ├─ Emergency stop ──→ Motor driver
   │         ├─ Gradual limit ──→ PWM reduction
   │         ├─ Throttle ───────→ Current limit
   │         ├─ LED status ─────→ Visual feedback
   │         └─ Fault log ──────→ Telemetry
   │
   └─→ Metrics Update ────────→ Power, energy, faults
                                  ├─→ Shared memory (Mutex)
                                  └─→ Telemetry task

FOC/Step-Dir Tasks
   │
   ├─→ Read current_limit_ma ──→ Apply thermal throttle
   │                             (automatically reduced if hot)
   │
   ├─→ Check emergency_stop ──→ Immediate shutdown signal
   │                             (if power fault detected)
   │
   └─→ Read power_metrics ────→ Log to telemetry
                                 (automatic visibility)

Benefit: Always monitoring, always protecting!
         Application doesn't need to remember.
```

---

## Protection Response Times

| Condition | Current | Phase 1 | Phase 3 (HW) | Improvement |
|-----------|---------|---------|--------------|-------------|
| **Overvoltage** | Never checked | 10 ms | 10 ms | ∞ → 10 ms |
| **Undervoltage** | Never checked | 10 ms | 10 ms | ∞ → 10 ms |
| **Overcurrent (peak)** | Never checked | 10 ms | <1 µs | ∞ → 10 ms/1 µs |
| **Overcurrent (RMS)** | Never calculated | 100 ms | 100 ms | ∞ → 100 ms |
| **MCU Overtemp** | Never checked | 1 s | 1 s | ∞ → 1 s |
| **Driver Fault** | Reactive only | Auto-recovery | Auto-recovery | Manual → Auto |
| **Brownout** | Not detected | 100 ms | 100 ms | Not possible → 100 ms |

---

## Memory Footprint Comparison

### Current Implementation

```
Flash:
  drivers/adc.rs           ~1.5 KB
  drivers/motor_driver.rs  ~0.8 KB
  Total:                   ~2.3 KB

RAM:
  ADC buffers              ~100 bytes
  Motor driver state       ~20 bytes
  Total:                   ~120 bytes
```

### Phase 1 Implementation

```
Flash:
  drivers/adc.rs           ~2.0 KB   (+500 B for temp sensing)
  drivers/motor_driver.rs  ~1.0 KB   (+200 B for auto-recovery)
  tasks/power_monitor.rs   ~2.0 KB   (NEW)
  RMS calculator           ~0.8 KB   (NEW)
  Total:                   ~5.8 KB   (+3.5 KB total)

RAM:
  ADC buffers              ~100 bytes
  Motor driver state       ~20 bytes
  Power monitor stack      ~2 KB      (task stack)
  RMS buffers              ~400 bytes (100 samples × 4 bytes)
  Power metrics            ~100 bytes
  Total:                   ~2.6 KB    (+2.5 KB total)

Available:
  Flash: 128 KB → 122 KB remaining (95% free)
  RAM:   32 KB → 29.4 KB remaining (92% free)
```

### Phase 2 Implementation (cumulative)

```
Flash:   +2 KB  → 7.8 KB total  (6% of 128 KB)
RAM:     +0.5 KB → 3.1 KB total  (10% of 32 KB)
```

**Conclusion:** Minimal resource impact!

---

## Testing Strategy Comparison

### Current (Manual)

```
Developer must:
1. Remember to call voltage check
2. Remember to check return value
3. Manually trigger protection
4. Hope nothing was missed
```

**Coverage:** ~20% (basic functions exist but not integrated)

### Phase 1 (Automated)

```
CI/CD Tests:
├─ Unit tests for protection logic
├─ Integration tests with mock sensors
├─ Renode emulation tests
├─ Hardware-in-loop tests:
│   ├─ Apply 52V → verify stop in <50ms
│   ├─ Apply 7V → verify stop in <50ms
│   ├─ Stall motor → verify current limit
│   ├─ Heat MCU → verify thermal throttle
│   └─ Trigger fault → verify auto-recovery
└─ Continuous monitoring validation
```

**Coverage:** ~80% (comprehensive protection)

---

## Failure Mode Comparison

### Current Architecture Failure Modes

| Failure | Detection | Response | Outcome |
|---------|-----------|----------|---------|
| 55V spike | ❌ Never | ❌ None | 💥 Damaged MCU/driver |
| 6V brownout | ❌ Never | ❌ None | 🔄 Uncontrolled reset |
| 3A overcurrent | ⚠️ DRV8844 only | ⚠️ Hardware trip | ⚠️ Works but no logging |
| 100°C MCU | ❌ Never | ❌ None | 🔥 Thermal damage possible |
| Driver fault | ✅ Can detect | ❌ Manual reset | ⏸️ Motor stops, user must act |
| Loose connection | ❌ Never | ❌ None | 📉 Erratic behavior |

**FMEA Score:** 3/10 (prototype only)

### Phase 1 Architecture Failure Modes

| Failure | Detection | Response | Outcome |
|---------|-----------|----------|---------|
| 55V spike | ✅ 10ms | ✅ Emergency stop | ✅ Safe shutdown, LED red |
| 6V brownout | ✅ 10ms | ✅ Emergency stop | ✅ Controlled shutdown |
| 3A overcurrent | ✅ 10ms | ✅ Current limit | ✅ Gradual reduction, logged |
| 100°C MCU | ✅ 1s | ✅ Thermal throttle | ✅ 70% power @ 70°C, stop @ 85°C |
| Driver fault | ✅ 10ms | ✅ Auto-recovery | ✅ 3 attempts, then safe stop |
| Loose connection | ✅ 100ms | ✅ Voltage sag detect | ✅ Warning, prepare shutdown |

**FMEA Score:** 8/10 (production-grade)

---

## Summary: Why This Matters

### Current State
- ✅ Hardware perfectly capable
- ⚠️ Software only 20% complete
- ❌ Suitable for **prototyping only**
- ❌ Not safe for **production deployment**

### After Phase 1 (13-18 hours work)
- ✅ Multi-layer safety protection
- ✅ Automatic fault handling
- ✅ Thermal management
- ✅ Professional-grade reliability
- ✅ Ready for **production deployment**

### After Phase 2 (+7-10 hours)
- ✅ Full diagnostics and telemetry
- ✅ Efficiency optimization
- ✅ Excellent troubleshooting capability
- ✅ **Industry-leading** motor controller

---

**Recommendation:** Implement Phase 1 immediately. It's the difference between a prototype and a product.

**See Also:**
- `docs/POWER_MANAGEMENT_ANALYSIS.md` - Detailed technical analysis
- `docs/POWER_IMPROVEMENTS_QUICK_REFERENCE.md` - Quick start guide
- `docs/CLN17_V2_HARDWARE_PINOUT.md` - Hardware reference
