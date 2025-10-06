# 🚀 iRPC v2.0 Phase 2 - Streaming Telemetry

**Date:** 2025-10-06 (Updated)  
**Status:** Phase 1 ✅ COMPLETE | Phase 2 Ready to Start  
**Branch:** `main` (Phase 1 merged)

---

## 📋 Current Status: EXCELLENT ✅

```
✅ Phase 1 COMPLETE - Motion Profiling (100%)
   - Motion planner with trapezoidal & S-curve (704 lines)
   - SetTargetV2 protocol (42 bytes, CAN-FD compatible)
   - FOC integration (10 kHz real-time)
   - 36 tests passing (14 unit + 22 integration)
   - 1,400+ lines documentation
   
✅ Build Status: PASSING (0 warnings)
✅ Performance: 5x better than targets
✅ Backward Compatibility: 100% maintained
```

**Phase 1 Achievements:**
- 📊 2,313 lines of production code
- 🎯 60% vibration reduction (S-curve)
- ⚡ 200 µs motion planning (target: < 1 ms)
- 🔧 50% mechanical wear reduction
- 📚 Complete protocol specification

---

## 🎯 Current Task: Phase 2 - Streaming Telemetry

### **Goal**

Implement high-frequency telemetry streaming for real-time monitoring and diagnostics.

**Key Features:**
1. 🔄 **High-frequency streaming** - 1 kHz position/velocity feedback
2. 📊 **Performance metrics** - FOC loop timing, CPU usage
3. 🔍 **Diagnostic data** - Current, temperature, load estimation
4. 📈 **Configurable modes** - On-demand, periodic, streaming, adaptive

**Expected Duration:** 2 weeks (80 hours)

### **Deliverables**

- Enhanced telemetry payloads with rich data
- Streaming mode support (1 kHz)
- Bandwidth optimization for CAN-FD
- Telemetry configuration commands
- 20+ new tests for streaming
- Updated protocol documentation

---

## 📂 Project Structure (Updated)

```
joint_firmware/
├── src/
│   └── firmware/
│       ├── tasks/
│       │   ├── can_comm.rs        ← TO ENHANCE (streaming)
│       │   └── foc.rs              ← ADD telemetry collection
│       ├── control/
│       │   ├── motion_planner.rs   ✅ COMPLETE (Phase 1)
│       │   └── observer.rs         ← TO USE (load estimation)
│       └── irpc_integration.rs    ← ADD telemetry handling
│
├── renode/
│   ├── tests/
│   │   ├── motion_planning.robot   ✅ COMPLETE (22 tests)
│   │   └── telemetry_streaming.robot  ← TO CREATE
│   └── peripherals/               ← TO ENHANCE (telemetry mock)
│
├── docs/
│   ├── IRPC_V2_PROTOCOL.md        ✅ Phase 1 complete
│   └── IRPC_V2_TELEMETRY.md       ← TO CREATE
│
├── PHASE_1_COMPLETE.md            ✅ Achievement summary
└── SESSION_SUMMARY.md             ✅ Session log
```

### **Key Files to Modify**

1. **iRPC Library** (`../../iRPC/` - sibling workspace):
   - `src/protocol.rs` - Add telemetry payloads
   - `src/joint.rs` - Add telemetry state management

2. **Firmware**:
   - `src/firmware/irpc_integration.rs` - Telemetry generation
   - `src/firmware/tasks/foc.rs` - Data collection in FOC loop
   - `src/firmware/tasks/can_comm.rs` - Streaming logic

3. **Tests**:
   - `renode/tests/telemetry_streaming.robot` - NEW FILE (20+ tests)
   - Update existing tests for telemetry verification

---

## 🔧 Phase 2 Detailed Tasks

### **Task 1: Enhanced Telemetry Payloads** (15 hours)

**File:** `../../iRPC/src/protocol.rs`

**What to add:**

```rust
/// Comprehensive telemetry stream (v2.0)
pub struct TelemetryStream {
    // Timestamp
    pub timestamp_us: u64,              // Microseconds since boot
    
    // Motion state
    pub position: f32,                  // Degrees
    pub velocity: f32,                  // Degrees/second
    pub acceleration: f32,              // Degrees/second² (calculated)
    
    // FOC state
    pub current_d: f32,                 // D-axis current (A)
    pub current_q: f32,                 // Q-axis current (A)
    pub voltage_d: f32,                 // D-axis voltage (V)
    pub voltage_q: f32,                 // Q-axis voltage (V)
    
    // Derived metrics
    pub torque_estimate: f32,           // Newton-meters
    pub power: f32,                     // Watts
    pub load_percent: f32,              // 0-100%
    
    // Performance
    pub foc_loop_time_us: u16,          // FOC loop execution time
    pub temperature_c: f32,             // Temperature (Celsius)
    
    // Status flags
    pub warnings: u16,                  // Warning flags
    pub trajectory_active: bool,        // Following trajectory?
}

/// Telemetry configuration
pub struct ConfigureTelemetryPayload {
    pub mode: TelemetryMode,
    pub rate_hz: u16,                   // Update rate (for Periodic)
    pub change_threshold: f32,          // Threshold (for OnChange)
}

/// Telemetry modes
pub enum TelemetryMode {
    OnDemand,           // Send only on request
    Periodic(u16),      // Send every N ms
    Streaming,          // Continuous at max rate (1 kHz)
    OnChange(f32),      // Send when value changes > threshold
    Adaptive,           // Adjust rate based on motion activity
}

/// Add to Payload enum
pub enum Payload {
    // ... existing variants
    
    // Telemetry (v2.0)
    TelemetryStream(TelemetryStream),
    ConfigureTelemetry(ConfigureTelemetryPayload),
    RequestTelemetry,                   // On-demand request
}
```

**Size Analysis:**
- TelemetryStream: ~60 bytes (fits in CAN-FD frame)
- Configure: ~8 bytes
- Efficient binary serialization (postcard)

### **Task 2: Telemetry Collection** (20 hours)

**File:** `src/firmware/tasks/foc.rs`

**What to implement:**

```rust
pub struct TelemetryCollector {
    // Accumulation buffers
    position_samples: RingBuffer<I16F16, 10>,
    velocity_samples: RingBuffer<I16F16, 10>,
    current_d_samples: RingBuffer<I16F16, 10>,
    current_q_samples: RingBuffer<I16F16, 10>,
    
    // Timing
    last_sample_time_us: u64,
    foc_loop_time_us: u16,
    
    // Configuration
    mode: TelemetryMode,
    rate_hz: u16,
}

impl TelemetryCollector {
    /// Collect data in FOC loop (called at 10 kHz)
    pub fn collect_sample(
        &mut self,
        position: I16F16,
        velocity: I16F16,
        currents: &Currents,
        voltages: &Voltages,
    );
    
    /// Check if telemetry should be sent
    pub fn should_send(&self, current_time_us: u64) -> bool;
    
    /// Generate telemetry payload
    pub fn generate_telemetry(&self) -> TelemetryStream;
}
```

**Optimization:**
- Use ring buffers for averaging (reduce noise)
- Minimal overhead in FOC loop (< 5 µs)
- Efficient fixed-point to float conversion

### **Task 3: Streaming Logic** (15 hours)

**File:** `src/firmware/tasks/can_comm.rs`

**What to add:**

```rust
pub struct TelemetryStreamer {
    collector: TelemetryCollector,
    config: TelemetryConfig,
    last_send_time_us: u64,
    send_interval_us: u64,
}

impl TelemetryStreamer {
    /// Process telemetry configuration command
    pub fn configure(&mut self, config: &ConfigureTelemetryPayload);
    
    /// Update and potentially send telemetry
    pub async fn update(&mut self, can: &mut CanFd) {
        if self.collector.should_send(current_time_us()) {
            let telemetry = self.collector.generate_telemetry();
            let msg = create_telemetry_message(telemetry);
            can.send_message(&msg).await?;
        }
    }
}
```

**Modes Implementation:**

1. **OnDemand** - Only when RequestTelemetry received
2. **Periodic(rate)** - Timer-based sending
3. **Streaming** - Maximum rate (1 kHz)
4. **OnChange(threshold)** - When value changes significantly
5. **Adaptive** - Fast during motion, slow when idle

### **Task 4: Bandwidth Optimization** (10 hours)

**CAN-FD Bandwidth Analysis:**

```
CAN-FD data rate: 5 Mbps
Message overhead: ~20 bytes (header + CRC)
Telemetry payload: 60 bytes
Total per message: 80 bytes = 640 bits

1 kHz rate: 640 kbps (12.8% of bandwidth)
✅ Sustainable with room for commands
```

**Optimizations:**
- Delta encoding for slow-changing values
- Configurable field selection
- Compression for logged data
- Adaptive rate based on CAN load

### **Task 5: Integration & Testing** (20 hours)

**File:** `renode/tests/telemetry_streaming.robot`

**Tests to write:**

```robot
*** Test Cases ***

Should Configure Telemetry Mode
    [Documentation]         Configure telemetry streaming mode
    [Tags]                  telemetry  configuration
    
    Configure Telemetry Mode    mode=Streaming    rate=1000
    Verify Telemetry Active

Should Stream Telemetry At 1kHz
    [Documentation]         Verify 1 kHz streaming rate
    [Tags]                  telemetry  streaming  performance
    
    Configure Telemetry Mode    mode=Streaming
    ${messages}=    Collect Telemetry    duration=1s
    ${count}=    Get Length    ${messages}
    Should Be True    900 <= ${count} <= 1100    # Allow 10% margin

Should Include Motion State In Telemetry
    [Documentation]         Verify position/velocity data
    [Tags]                  telemetry  motion
    
    Send SetTarget V2    90.0
    Configure Telemetry Mode    mode=Streaming
    ${telemetry}=    Get Telemetry Sample
    Should Contain    ${telemetry}    position
    Should Contain    ${telemetry}    velocity

Should Include FOC State In Telemetry
    [Documentation]         Verify current/voltage data
    [Tags]                  telemetry  foc
    
    ${telemetry}=    Get Telemetry Sample
    Should Have Field    ${telemetry.current_d}
    Should Have Field    ${telemetry.current_q}

Should Calculate Derived Metrics
    [Documentation]         Verify torque/power calculation
    [Tags]                  telemetry  metrics
    
    ${telemetry}=    Get Telemetry Sample
    Should Have Field    ${telemetry.torque_estimate}
    Should Have Field    ${telemetry.power}
    Should Have Field    ${telemetry.load_percent}

Should Handle OnDemand Mode
    [Documentation]         Only send when requested
    [Tags]                  telemetry  on-demand
    
    Configure Telemetry Mode    mode=OnDemand
    Request Telemetry
    ${telemetry}=    Wait For Telemetry    timeout=100ms
    Should Not Be Empty    ${telemetry}

Should Handle Periodic Mode
    [Documentation]         Send at configured rate
    [Tags]                  telemetry  periodic
    
    Configure Telemetry Mode    mode=Periodic    rate=100
    ${messages}=    Collect Telemetry    duration=1s
    ${count}=    Get Length    ${messages}
    Should Be True    90 <= ${count} <= 110

Should Handle OnChange Mode
    [Documentation]         Send only on significant changes
    [Tags]                  telemetry  on-change
    
    Configure Telemetry Mode    mode=OnChange    threshold=5.0
    ${count_idle}=    Count Telemetry Messages    duration=1s
    
    Send SetTarget V2    90.0
    ${count_motion}=    Count Telemetry Messages    duration=1s
    
    Should Be True    ${count_motion} > ${count_idle}

Should Handle Adaptive Mode
    [Documentation]         Adapt rate to motion activity
    [Tags]                  telemetry  adaptive
    
    Configure Telemetry Mode    mode=Adaptive
    # During motion: high rate
    Send SetTarget V2    90.0
    ${rate_motion}=    Measure Telemetry Rate    duration=1s
    
    # During idle: low rate
    Wait For Motion Complete
    ${rate_idle}=    Measure Telemetry Rate    duration=1s
    
    Should Be True    ${rate_motion} > ${rate_idle} * 3

Should Report FOC Loop Timing
    [Documentation]         Monitor FOC performance
    [Tags]                  telemetry  performance
    
    ${telemetry}=    Get Telemetry Sample
    ${loop_time}=    Get    ${telemetry.foc_loop_time_us}
    Should Be True    ${loop_time} < 100    # < 100 µs (10 kHz)

Should Detect Trajectory Following
    [Documentation]         Report trajectory active status
    [Tags]                  telemetry  trajectory
    
    Send SetTarget V2    90.0
    ${telemetry}=    Get Telemetry Sample
    Should Be True    ${telemetry.trajectory_active}

Should Report Load Estimation
    [Documentation]         Estimate mechanical load
    [Tags]                  telemetry  load
    
    # Apply load via mock
    Set Motor Load    50    # 50% load
    ${telemetry}=    Get Telemetry Sample
    Should Be True    40 <= ${telemetry.load_percent} <= 60

Should Handle Bandwidth Limits
    [Documentation]         Gracefully handle CAN saturation
    [Tags]                  telemetry  bandwidth
    
    # Saturate CAN with commands
    FOR    ${i}    IN RANGE    100
        Send SetTarget V2    ${i}
    END
    
    # Telemetry should still work
    ${telemetry}=    Get Telemetry Sample    timeout=1s
    Should Not Be Empty    ${telemetry}

... (8+ more tests)
```

---

## 🚨 Important Guidelines (Unchanged)

### **Development Principles**

1. ✅ **Clean Code** - SOLID, DRY, KISS principles
2. ✅ **Test First** - Write tests before/with implementation
3. ✅ **Incremental** - Small, atomic commits
4. ✅ **Documentation** - Comment complex logic
5. ✅ **Performance** - Profile critical paths (< 5 µs in FOC loop)
6. ✅ **Safety** - Validate all inputs, handle errors

### **Performance Requirements**

| Metric | Target | Critical |
|--------|--------|----------|
| Telemetry collection | < 5 µs | FOC loop overhead |
| Telemetry generation | < 50 µs | Message creation |
| Streaming rate | 1 kHz | Maximum bandwidth |
| CAN bandwidth usage | < 20% | Leave room for commands |
| Memory per stream | < 5 KB | Embedded constraints |

### **Error Handling**

```rust
// ✅ GOOD: Non-blocking telemetry
pub fn send_telemetry(&mut self) -> Result<(), TelemetryError> {
    match self.try_send() {
        Ok(_) => Ok(()),
        Err(TelemetryError::CanBusy) => {
            // Drop this sample, continue
            defmt::debug!("Telemetry dropped: CAN busy");
            Ok(())
        }
        Err(e) => Err(e),
    }
}

// ❌ BAD: Blocking or panicking
pub fn send_telemetry(&mut self) {
    self.can.send_blocking(&msg).unwrap();  // Blocks FOC!
}
```

---

## 📊 Phase 2 Success Criteria

### Functionality ✅
- [ ] TelemetryStream payload implemented
- [ ] All telemetry modes working (OnDemand, Periodic, Streaming, OnChange, Adaptive)
- [ ] FOC integration with < 5 µs overhead
- [ ] 1 kHz streaming achieved
- [ ] Load estimation working

### Quality ✅
- [ ] 20+ tests passing
- [ ] No FOC loop timing violations
- [ ] CAN bandwidth < 20% at 1 kHz
- [ ] No memory leaks

### Performance ✅
- [ ] Telemetry collection < 5 µs
- [ ] Message generation < 50 µs
- [ ] 1 kHz sustained rate
- [ ] Adaptive mode reduces bandwidth by 70% when idle

### Documentation ✅
- [ ] Protocol documentation updated
- [ ] API docs for telemetry
- [ ] Usage examples provided
- [ ] Performance analysis documented

---

## 🔄 Git Workflow (Same as Phase 1)

```bash
# 1. Create Phase 2 branch
git checkout -b feature/irpc-v2-telemetry

# 2. Implement telemetry
# - Incremental commits
# - Each commit functional

# 3. Example commit:
git commit -m "feat(telemetry): Add TelemetryStream payload

- Implement comprehensive telemetry structure
- Add position, velocity, FOC state
- Include derived metrics (torque, power, load)
- Performance metrics (loop time)
- 60 bytes, fits in CAN-FD frame

Refs: IRPC_V2_PROTOCOL.md Phase 2"

# 4. Merge to main when complete
git checkout main
git merge --no-ff feature/irpc-v2-telemetry
git branch -d feature/irpc-v2-telemetry
```

---

## 📚 Technical References

### **Phase 1 Documents** (Completed)
- ✅ `docs/IRPC_V2_PROTOCOL.md` - Motion planning spec
- ✅ `PHASE_1_COMPLETE.md` - Phase 1 achievements
- ✅ `SESSION_SUMMARY.md` - Implementation log

### **Phase 2 Research**
- `docs/IRPC_EVOLUTION_RESEARCH.md` - Section 4.2: Telemetry
- TMC5160T datasheet - Diagnostic features
- CAN-FD bandwidth calculations

### **Telemetry Design Considerations**

**Bandwidth Math:**
```
CAN-FD: 5 Mbps data phase
Message: 80 bytes = 640 bits
1 kHz rate: 640 kbps (12.8% bandwidth)

Room for:
- Commands: ~100/sec (1.3%)
- Other traffic: 86% available
✅ Sustainable
```

**FOC Loop Integration:**
```
FOC loop: 100 µs period (10 kHz)
Telemetry collection budget: < 5 µs (5% overhead)

Allowed operations:
✅ Read sensor values (< 1 µs)
✅ Ring buffer update (< 1 µs)
✅ Simple calculations (< 2 µs)
✅ Flags/counters (< 1 µs)

NOT allowed:
❌ Float conversions (defer to send time)
❌ CAN transmission (async task)
❌ Complex calculations (defer)
```

---

## 💬 Communication Style (Unchanged)

**When working with me:**

1. ✅ **Start with planning** - Design approach first
2. ✅ **Show your work** - Explain decisions
3. ✅ **Incremental progress** - Small commits, updates
4. ✅ **Test thoroughly** - Verify each feature
5. ✅ **Document as you go** - Keep docs current

**I prefer:**
- 📊 Code over talk
- 🎯 Direct solutions
- ⚡ Fast iteration
- 🧪 Tests as proof

---

## 🚀 Let's Start Phase 2!

**Your first message should be:**

1. ✅ Confirm you understand Phase 2 goals
2. ✅ Outline approach for telemetry payloads
3. ✅ Create feature branch
4. ✅ Start coding!

**Example:**
```
Ready to implement Phase 2: Streaming Telemetry! 🚀

Approach:
1. Create feature/irpc-v2-telemetry branch
2. Add TelemetryStream payload to protocol
3. Implement TelemetryCollector in FOC loop
4. Add streaming logic to CAN task
5. Create comprehensive tests
6. Optimize for < 5 µs collection overhead

Starting with protocol enhancement...
```

---

## 📎 Quick Commands (Updated)

```bash
# Build firmware
cargo build --release --features renode-mock

# Run tests (including Phase 1)
cargo test
renode-test renode/tests/

# Check Phase 1 status
git log --oneline -5

# Performance profiling
cargo build --release --features renode-mock,profiling

# Documentation
cargo doc --open

# Git workflow (Phase 2)
git checkout -b feature/irpc-v2-telemetry
git commit -m "feat(telemetry): <description>"
git push origin feature/irpc-v2-telemetry
```

---

## 🎯 Phase 1 vs Phase 2

| Aspect | Phase 1 ✅ | Phase 2 🚀 |
|--------|-----------|-----------|
| **Focus** | Motion planning | Telemetry streaming |
| **Complexity** | Algorithms | Real-time streaming |
| **Performance** | Planning: < 1 ms | Collection: < 5 µs |
| **Tests** | 22 integration | 20+ integration |
| **Impact** | Better motion | Better observability |
| **Lines** | ~2,300 added | ~1,500 expected |

---

**ВПЕРЁД! Phase 2: Streaming Telemetry! 🚀💪**

**Phase 1 Foundation is solid. Time to add real-time observability!** 📡

---

_Last Updated: 2025-10-06 after Phase 1 completion_
