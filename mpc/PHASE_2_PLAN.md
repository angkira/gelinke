# MPC Phase 2: Rust/Embedded Implementation Plan

**Status:** 🚧 In Progress
**Started:** 2025-10-10
**Phase 1 Results:** 1.655° RMS tracking, 3.8ms solve time (Python/OSQP)

---

## Overview

Phase 2 implements MPC in Rust for embedded deployment on STM32G431CB (Cortex-M4F @ 170 MHz).

**Key Challenge:** Python solver is 38x too slow for 10 kHz (3.8ms vs 100µs target).

**Solution:** Hybrid architecture with 1 kHz MPC outer loop + 10 kHz PID inner loop.

---

## Architecture: Hybrid MPC-PID Cascade

```text
┌─────────────────────────────────────────────────┐
│  1 kHz MPC Outer Loop (Floating Point)         │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  • Solves QP optimization (target: <500µs)     │
│  • Predicts 25ms ahead (N=25 @ 1kHz)          │
│  • Generates position/velocity setpoints       │
│  • Handles constraints & optimal trajectory    │
└──────────────┬──────────────────────────────────┘
               │ pos_ref, vel_ref
               ↓
┌─────────────────────────────────────────────────┐
│  10 kHz PID Inner Loop (Fixed Point I16F16)    │
│  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━  │
│  • Position P controller (6.0)                 │
│  • Velocity PID (kp=3.5, ki=3.5, kd=1.4)      │
│  • FOC current control                         │
│  • Hardware limit enforcement                  │
└─────────────────────────────────────────────────┘
```

### Benefits of Hybrid Approach

✅ **10x longer compute time budget** (1ms vs 100µs)
✅ **Keeps proven PID inner loop** (fast, deterministic, safe)
✅ **Can use floating-point math** (M4F FPU available)
✅ **Easier to debug and validate**
✅ **Still gets MPC benefits** (constraint handling, preview control)

---

## Implementation Options

### Option A: Rust OSQP Bindings 🎯 **RECOMMENDED**

**Use Case:** General-purpose embedded MPC with good performance

**Approach:**
1. Add `osqp` crate to Cargo.toml (check if no_std compatible)
2. If not no_std: Use `osqp-sys` with custom allocator
3. Port Python MPC formulation to Rust
4. Optimize QP problem structure

**Pros:**
- Same solver as validated Python prototype
- Mature, well-tested algorithm
- Warm-start support for speed
- Rust type safety

**Cons:**
- May need heap allocation (manageable with `embedded-alloc`)
- Binary size ~50-100 KB
- Solve time likely 500µs - 2ms (acceptable at 1 kHz)

**Estimated Timeline:** 3-4 days

---

### Option B: Code-Generated QP Solver ⚡ **FASTEST**

**Use Case:** Need <100µs solve time at 10 kHz

**Approach:**
1. Use CVXGEN, acados, or FORCES Pro to generate C code
2. Export Python problem structure to code generator
3. Generate fixed-structure, optimized C solver
4. Call via Rust FFI

**Pros:**
- Can achieve 50-200µs solve times
- No dynamic memory allocation
- Tailored to specific problem size
- Supports 10 kHz operation

**Cons:**
- Requires commercial tool (FORCES) or complex setup (acados)
- Fixed problem structure (can't change N or constraints easily)
- Code generation in toolchain
- Harder to debug

**Estimated Timeline:** 1-2 weeks (with learning curve)

---

### Option C: Simplified Explicit MPC 🔬 **EXPERIMENTAL**

**Use Case:** Very small horizons (N<10) with specific structure

**Approach:**
1. Pre-compute optimal control law offline
2. Store as lookup table or piecewise-linear function
3. Evaluate at runtime (microseconds)

**Pros:**
- Extremely fast (<10µs)
- No optimization at runtime
- Minimal code size

**Cons:**
- Only works for small N (limited preview)
- Complex offline computation
- Less flexible
- May not handle constraints well

**Estimated Timeline:** 1 week (research + implementation)

---

### Option D: Rust from Scratch 🛠️ **EDUCATIONAL**

**Use Case:** Learning, custom requirements, no dependencies

**Approach:**
1. Implement primal-dual interior point method in Rust
2. Exploit sparse matrix structure
3. Use fixed-point or floating-point as needed

**Pros:**
- Full control over algorithm
- No external dependencies
- Can optimize for specific hardware
- Great learning experience

**Cons:**
- Weeks of development time
- Hard to match OSQP performance
- Numerical stability challenges
- Likely slower than mature solvers

**Estimated Timeline:** 2-4 weeks

---

## Recommended Path: Option A (Rust OSQP)

Start with Rust OSQP bindings for fastest path to working embedded MPC.

### Phase 2a: Basic Rust MPC (Days 1-2)

- [x] Create MPC module structure (`src/firmware/control/mpc.rs`)
- [ ] Research `osqp` crate compatibility with no_std
- [ ] Add OSQP dependency to Cargo.toml
- [ ] Port Python MPC formulation to Rust
- [ ] Test on host (cargo test)

### Phase 2b: Embedded Integration (Days 3-4)

- [ ] Setup 1 kHz async task for MPC
- [ ] Connect MPC output to position controller setpoint
- [ ] Add telemetry for MPC solve time and status
- [ ] Test in Renode simulation

### Phase 2c: Optimization (Days 5-7)

- [ ] Reduce horizon if needed (N=25 → N=15)
- [ ] Tune OSQP settings (eps_abs, eps_rel, max_iter)
- [ ] Profile with `defmt` to find bottlenecks
- [ ] Target <500µs mean solve time

---

## Technical Details

### QP Problem Structure

The MPC optimization is a sparse Quadratic Program (QP):

```
minimize:  (1/2) x'Px + q'x
subject to: l ≤ Ax ≤ u
```

Where:
- **x**: Decision variables [x_0, ..., x_N, u_0, ..., u_{N-1}]
  - States: position, velocity, acceleration over horizon
  - Controls: jerk over horizon
  - Total size: 3*(N+1) + N = 4N + 3 variables

- **P**: Hessian matrix (sparse, block-diagonal structure)
  - Q blocks for state cost
  - R blocks for control cost
  - Size: (4N+3) × (4N+3), but sparse (only ~12N non-zeros)

- **A**: Constraint matrix (sparse, dynamics + bounds)
  - Dynamics: x[k+1] = A*x[k] + B*u[k]
  - Bounds: v_min ≤ vel ≤ v_max, etc.
  - Size: ~(6N) × (4N+3), sparse

### Problem Size Analysis

For N=25 (25ms lookahead @ 1kHz):

| Parameter | Value | Memory |
|-----------|-------|--------|
| Variables | 103 | 412 bytes (f32) |
| Constraints | 150 | - |
| P matrix non-zeros | ~300 | 1.2 KB |
| A matrix non-zeros | ~450 | 1.8 KB |
| **Total QP data** | - | **~5-10 KB** |

### Memory Budget (STM32G431CB)

- **RAM:** 32 KB total
- **Stack:** ~8 KB (Embassy)
- **Static:** ~10 KB (firmware)
- **Available:** ~14 KB

**MPC Memory Usage:**
- QP problem data: 5-10 KB
- OSQP workspace: 5-10 KB
- **Total: ~15-20 KB** ⚠️ **TIGHT BUT FEASIBLE**

### Optimization Strategies

1. **Reduce horizon:** N=25 → N=15 saves ~40% memory
2. **Increase dt:** 1ms → 2ms (run at 500 Hz)
3. **Use f32 not f64:** Already assumed above
4. **Exploit sparsity:** Only store non-zero elements
5. **Warm-start:** Use previous solution as initial guess

---

## Performance Targets

| Metric | Target | Stretch Goal |
|--------|--------|--------------|
| **Solve Time** | <500 µs | <200 µs |
| **Success Rate** | >95% | >99% |
| **Tracking RMS** | <1.5° | <1.0° |
| **Memory Usage** | <20 KB | <15 KB |

---

## Integration with Existing Firmware

### Async Task Structure

```rust
#[embassy_executor::task]
async fn mpc_control_task(
    mut mpc: MPCController,
    position_rx: Receiver<PositionState>,
    setpoint_tx: Sender<MPCSetpoint>,
) {
    let mut ticker = Ticker::every(Duration::from_hz(1000));

    loop {
        ticker.next().await;

        // Get current state from FOC control
        let state = position_rx.try_recv().unwrap_or_default();

        // Generate reference trajectory (e.g., from S-curve planner)
        let reference = generate_reference_trajectory(25);

        // Solve MPC
        let output = mpc.solve(state, &reference);

        // Send setpoints to PID controller
        let setpoint = MPCSetpoint {
            position: output.predicted_position,
            velocity: output.predicted_velocity,
        };

        setpoint_tx.send(setpoint).await;

        // Telemetry
        defmt::trace!("MPC solve: {}us", output.solve_time_us);
    }
}
```

### Connecting to Position Controller

Modify `position.rs` to accept MPC setpoints:

```rust
pub enum PositionSetpoint {
    /// Fixed target position
    Fixed(I16F16),

    /// MPC-generated trajectory setpoint
    Trajectory { position: I16F16, velocity: I16F16 },
}
```

---

## Testing Strategy

### Phase 2a: Host Testing

```bash
# Run unit tests
cargo test --lib

# Run MPC solver benchmark
cargo test --release mpc_benchmark -- --nocapture
```

### Phase 2b: Renode Simulation

```bash
# Build firmware with MPC
cargo build --release

# Run in Renode
./tools/renode/run_renode.sh

# Monitor MPC telemetry via defmt
```

### Phase 2c: Hardware Testing

1. Flash firmware to STM32G431CB
2. Connect to motor + encoder
3. Run system identification (use test_sequences.json)
4. Update model parameters
5. Test S-curve tracking with MPC
6. Compare vs PID baseline

---

## Risks and Mitigations

| Risk | Impact | Mitigation |
|------|--------|------------|
| **OSQP not no_std** | Can't compile | Use osqp-sys + custom allocator OR Option B |
| **RAM overflow** | Hard fault | Reduce N, use f32, profile memory |
| **Solve time >1ms** | Miss deadlines | Reduce N, tune OSQP, or accept 500 Hz |
| **Numerical instability** | Solver failures | Add regularization, check conditioning |
| **Worse than PID** | No benefit | Tune weights, increase horizon, add feedforward |

---

## Success Criteria

Phase 2 is complete when:

- [x] MPC module compiles and passes unit tests
- [ ] MPC runs on target hardware without crashes
- [ ] Solve time <500 µs (mean) at 1 kHz
- [ ] Tracking error ≤ 1.7° RMS (at least as good as PID)
- [ ] No memory overflows or hard faults
- [ ] Telemetry shows solver success rate >95%

---

## Next Steps After Phase 2

If Phase 2 succeeds → **Phase 3: Production Hardening**
- Safety monitoring (timeout detection, fallback to PID)
- Online parameter adaptation
- Multi-motor coordination
- Field testing

If Phase 2 struggles with timing → **Pivot to Option B (Code-Gen)**
- Use acados or FORCES for optimized solver
- Target <100µs for 10 kHz operation

If Phase 2 shows no improvement → **Stay with PID + S-curve**
- 1.903° RMS is already excellent
- Simpler, proven, production-ready
- Focus efforts elsewhere (e.g., friction compensation, thermal management)

---

*Last updated: 2025-10-10*
*Current: Phase 2a - Setting up Rust OSQP integration*
