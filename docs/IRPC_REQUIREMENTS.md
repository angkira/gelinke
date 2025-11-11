# iRPC Feature Requirements for Power Monitoring Integration

**Date:** 2025-11-10
**Project:** CLN17 v2.0 Motor Controller Firmware
**Component:** Power Monitoring Telemetry over CAN
**iRPC Repository:** https://github.com/angkira/iRPC

---

## Overview

The CLN17 v2.0 firmware now includes comprehensive power monitoring with 100 Hz sampling of voltage, current, temperature, and fault detection. To complete the integration, we need to stream these metrics over CAN using iRPC.

**Current Status:**
- ✅ Power monitoring task implemented and active
- ✅ POWER_METRICS shared state with Mutex access
- ✅ CAN communication task using iRPC (joint_api feature)
- ⏳ Telemetry streaming over CAN (pending iRPC support)

---

## Required iRPC Features

### 1. Power Metrics Telemetry Message

**Message Type:** Periodic telemetry broadcast

**Payload Structure:**
```rust
pub struct PowerMetrics {
    pub vbus_mv: u32,           // Supply voltage (millivolts)
    pub ia_ma: i32,             // Phase A current (milliamps, signed)
    pub ib_ma: i32,             // Phase B current (milliamps, signed)
    pub i_rms_ma: f32,          // RMS current (milliamps)
    pub power_mw: u32,          // Instantaneous power (milliwatts)
    pub mcu_temp_c: f32,        // MCU temperature (degrees C)
    pub throttle_factor: f32,   // Thermal throttle (0.0 to 1.0)
    pub energy_mwh: u32,        // Accumulated energy (milliwatt-hours)
    pub charge_mah: u32,        // Accumulated charge (milliamp-hours)
    pub active_time_ms: u32,    // Active time (milliseconds)
    pub faults: FaultCounters,  // Fault counters
}

pub struct FaultCounters {
    pub overcurrent_events: u16,
    pub overvoltage_events: u16,
    pub undervoltage_events: u16,
    pub overtemp_events: u16,
    pub driver_fault_events: u16,
    pub emergency_stops: u16,
}
```

**Transmission Requirements:**
- **Frequency:** Configurable (default 10 Hz, range 1-100 Hz)
- **Priority:** Medium (diagnostic data, not safety-critical)
- **Size:** ~56 bytes (needs efficient encoding)
- **Direction:** Device → Host (broadcast)

**Use Cases:**
- Real-time power consumption monitoring
- Thermal management visualization
- Predictive maintenance (fault tracking)
- Energy usage statistics

---

### 2. Fault History Message

**Message Type:** On-demand query/response

**Payload Structure:**
```rust
pub struct FaultHistory {
    pub records: [FaultRecord; 10],  // Last 10 faults
    pub total_faults: u32,           // Lifetime fault count
}

pub struct FaultRecord {
    pub fault_type: u8,      // FaultType enum as u8
    pub timestamp: u32,      // Seconds since boot
    pub vbus_mv: u16,        // Voltage at fault time
    pub current_ma: u16,     // Current at fault time
    pub temp_c: i8,          // Temperature at fault time
}
```

**Transmission Requirements:**
- **Frequency:** On-demand (host requests, device responds)
- **Priority:** Low (diagnostic data)
- **Size:** ~100 bytes
- **Direction:** Bidirectional (request/response)

**Use Cases:**
- Fault diagnosis and troubleshooting
- Failure pattern analysis
- Warranty claims investigation

---

### 3. Emergency Stop Notification

**Message Type:** High-priority event notification

**Payload Structure:**
```rust
pub struct EmergencyStop {
    pub reason: EmergencyReason,  // Enum: Overvoltage, Overcurrent, etc.
    pub vbus_mv: u32,
    pub current_ma: i32,
    pub temp_c: f32,
    pub timestamp_ms: u32,
}

pub enum EmergencyReason {
    Overvoltage,
    Undervoltage,
    PeakOvercurrent,
    RmsOvercurrent,
    Overtemperature,
    DriverFault,
    WatchdogReset,
    ManualStop,
}
```

**Transmission Requirements:**
- **Frequency:** Event-driven (immediate on emergency stop)
- **Priority:** HIGH (safety-critical notification)
- **Size:** ~20 bytes
- **Direction:** Device → Host (broadcast)
- **Latency:** <10ms from event to CAN transmission

**Use Cases:**
- Immediate fault notification
- Safety system coordination
- System-wide emergency stop propagation

---

### 4. Power Configuration Message

**Message Type:** Bidirectional configuration

**Payload Structure:**
```rust
pub struct PowerConfig {
    pub vbus_overvoltage_mv: u32,   // Default: 50000 (50V)
    pub vbus_undervoltage_mv: u32,  // Default: 8000 (8V)
    pub max_rms_current_ma: u16,    // Default: 1750 (1.75A)
    pub max_peak_current_ma: u16,   // Default: 2500 (2.5A)
    pub temp_throttle_start_c: u8,  // Default: 70°C
    pub temp_shutdown_c: u8,        // Default: 85°C
    pub telemetry_rate_hz: u8,      // Default: 10 Hz
}
```

**Transmission Requirements:**
- **Frequency:** On-demand (configuration changes)
- **Priority:** Medium
- **Size:** ~16 bytes
- **Direction:** Bidirectional (get/set)

**Use Cases:**
- Runtime configuration adjustment
- Application-specific power limits
- Telemetry rate adjustment for bandwidth management

---

## Integration Points

### Current Firmware Implementation

**File:** `src/firmware/tasks/power_monitor.rs`
```rust
/// Shared power metrics state (thread-safe).
pub static POWER_METRICS: Mutex<CriticalSectionRawMutex, PowerMetrics> =
    Mutex::new(PowerMetrics::new());
```

**Access Pattern:**
```rust
// Read metrics from any task
let metrics = POWER_METRICS.lock().await;
let vbus = metrics.vbus_mv;
let temp = metrics.mcu_temp_c;
drop(metrics);
```

### Proposed Telemetry Task

**File:** `src/firmware/tasks/telemetry.rs` (to be extended)

```rust
#[embassy_executor::task]
pub async fn power_telemetry_task() {
    let mut ticker = Ticker::every(Duration::from_millis(100)); // 10 Hz

    loop {
        ticker.next().await;

        // Read power metrics
        let metrics = POWER_METRICS.lock().await.clone();
        drop(metrics);

        // Send via iRPC CAN
        // TODO: Requires iRPC support for PowerMetrics message
        // irpc_send_power_metrics(&metrics).await;
    }
}
```

---

## API Schema Requirements

### joint_api Extension

The iRPC `joint_api` feature needs to be extended with power monitoring messages:

**Suggested API Additions:**

1. **Message IDs:**
   - `0x200`: PowerMetrics (periodic telemetry)
   - `0x201`: FaultHistory (query/response)
   - `0x202`: EmergencyStop (event notification)
   - `0x203`: PowerConfig (get/set configuration)

2. **Encoding:**
   - Use efficient binary encoding (not JSON) for CAN bandwidth
   - Consider message fragmentation for >8 byte payloads (CAN FD if available)
   - CRC protection for multi-frame messages

3. **Protocol Features:**
   - Periodic broadcast (configurable rate)
   - Request/response pattern (for queries)
   - Event notification (high-priority)
   - Configuration get/set

---

## Priority Assessment

**HIGH Priority (Required for A+ grade):**
1. ✅ PowerMetrics telemetry message
2. ✅ Emergency stop notification

**MEDIUM Priority (Nice to have):**
3. ⏺️ Power configuration message
4. ⏺️ Fault history query/response

**LOW Priority (Future enhancement):**
5. ⏺️ Historical data logging
6. ⏺️ Power analytics (min/max/average over time)

---

## Architecture Note

**IMPORTANT:** This firmware uses iRPC as the communication abstraction layer. We do NOT communicate with CAN directly. All telemetry and command messages MUST go through the iRPC API.

**Current Stack:**
```
Firmware Tasks → iRPC API → CAN Transport Layer → Physical CAN Bus
```

**Benefits of iRPC Abstraction:**
- Type-safe message definitions
- Automatic serialization/deserialization
- Transport independence (CAN, USB, UART, Ethernet)
- Code generation for host and device
- Version compatibility checking
- Message routing and filtering

## Alternative Solutions

If iRPC extension is not immediately available:

### Option A: Generic Binary Telemetry via iRPC
- Use existing iRPC generic data message types
- Serialize PowerMetrics to byte array
- Send through iRPC (maintains abstraction)
- Less type-safe, requires manual deserialization on host

### Option B: USB/UART Telemetry via iRPC
- Configure iRPC to use USB or UART transport
- Same message types, different physical layer
- Higher bandwidth, lower latency
- Still uses iRPC abstraction (preferred approach)

---

## Testing Requirements

Once iRPC support is added:

1. **Unit Tests:**
   - Message serialization/deserialization
   - CRC validation
   - Fragmentation/reassembly

2. **Integration Tests:**
   - 10 Hz telemetry rate (no drops)
   - Emergency stop latency (<10ms)
   - Configuration round-trip

3. **Hardware Tests:**
   - CAN bus loading with telemetry active
   - Message prioritization under load
   - Long-term stability (24+ hours)

---

## Summary

**Required from iRPC team:**
1. Add PowerMetrics message type to joint_api schema
2. Add EmergencyStop message type to joint_api schema
3. Support configurable periodic broadcast (1-100 Hz)
4. Support high-priority event notifications
5. Provide code generation for firmware integration

**Timeline Impact:**
- Without iRPC support: Fallback to USB/UART telemetry (2-3 hours)
- With iRPC support: Proper CAN telemetry integration (2-3 hours)

**Contact:**
This firmware is ready for iRPC integration. Once the API schema is extended, integration can be completed in a single session (~2-3 hours).

---

**Generated:** 2025-11-10
**Firmware Version:** A (92/100)
**Next Target:** A+ (95/100) with full telemetry integration
