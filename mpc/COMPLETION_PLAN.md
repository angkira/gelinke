# MPC Feature Completion and Testing Plan

**Status:** Phase 2a Complete (Embedded C code generated)
**Remaining:** Phase 2b-2c (Rust integration) + Phase 3 (Validation)
**Timeline:** 4-6 days for full completion
**Risk:** Medium (experimental feature, tight memory budget)

---

## Current Status: What's Done ✅

### Phase 1: Python Prototype (COMPLETE)
- [x] System identification framework (`system_identification.py`)
- [x] MPC controller implementation (`mpc_controller.py`)
- [x] Simulation validation (1.655° RMS, 13% improvement over PID)
- [x] Performance analysis (3.8ms solve time - too slow for 10 kHz)

### Phase 2a: Embedded Code Generation (COMPLETE)
- [x] OSQP embedded C solver generated (malloc-free, library-free)
- [x] 75KB static library compiled (`libemosqp.a`)
- [x] Test program validated (solver runs without errors)
- [x] Rust MPC module structure created (`src/firmware/control/mpc.rs`)
- [x] Feature flag setup (MPC is optional: `--features mpc`)
- [x] Build script for conditional linking (`build.rs`)

---

## What Remains: Phases 2b-3

### Phase 2b: Rust FFI Integration (2-3 days)

**Goal:** Call embedded OSQP solver from Rust safely

#### Tasks:
1. **Create FFI bindings** (4-6 hours)
   - [ ] Read `mpc/embedded_mpc/inc/public/osqp.h` to understand C API
   - [ ] Write Rust `extern "C"` declarations in `mpc.rs`
   - [ ] Create safe Rust wrappers around unsafe C calls
   - [ ] Handle memory layout (ensure C and Rust agree on struct sizes)

2. **Implement MPC solver interface** (6-8 hours)
   - [ ] Implement `MPCController::solve()` using OSQP FFI
   - [ ] Convert Rust state (I16F16) ↔ C arrays (float)
   - [ ] Update problem vectors (q, l, u) at each solve
   - [ ] Extract solution from C workspace
   - [ ] Add error handling for solver failures

3. **Test on host** (2-4 hours)
   - [ ] Write unit tests in `mpc.rs` (use `#[cfg(test)]`)
   - [ ] Test solver initialization
   - [ ] Test simple optimization problems
   - [ ] Validate solution accuracy vs Python prototype
   - [ ] Profile memory usage

**Deliverable:** `cargo test --features mpc` passes on host

---

### Phase 2c: Firmware Integration (1-2 days)

**Goal:** Integrate MPC into control loop without breaking PID

#### Tasks:
1. **Create MPC control task** (3-4 hours)
   - [ ] Add 1 kHz Embassy async task for MPC
   - [ ] Setup channels for state input / setpoint output
   - [ ] Generate reference trajectory (from motion planner)
   - [ ] Call MPC solver each cycle
   - [ ] Add timeout detection (fallback to PID if MPC too slow)

2. **Connect to position controller** (2-3 hours)
   - [ ] Modify `position.rs` to accept MPC trajectory setpoints
   - [ ] Add mode switching: PID-only vs MPC+PID
   - [ ] Ensure smooth transition between modes
   - [ ] Add telemetry for MPC status

3. **Test in Renode** (3-4 hours)
   - [ ] Build with `--features mpc`
   - [ ] Run in Renode simulation
   - [ ] Verify MPC task runs at 1 kHz
   - [ ] Check solve times (target <500µs)
   - [ ] Verify PID inner loop unaffected
   - [ ] Test mode switching

**Deliverable:** MPC runs in Renode without crashing

---

### Phase 3: Hardware Validation (2-3 days)

**Goal:** Prove MPC works on real hardware and improves performance

#### Tasks:
1. **System identification on hardware** (4-6 hours)
   - [ ] Flash firmware to STM32G431CB
   - [ ] Run test sequences (from `test_sequences.json`)
   - [ ] Collect step response data
   - [ ] Re-run `system_identification.py` on real data
   - [ ] Update `motor_model.json` with actual parameters
   - [ ] Regenerate embedded solver with real model
   - [ ] Rebuild firmware

2. **MPC performance testing** (4-6 hours)
   - [ ] Test S-curve tracking with MPC enabled
   - [ ] Measure RMS tracking error
   - [ ] Compare to PID baseline (1.903° target)
   - [ ] Profile solve times (should be <500µs)
   - [ ] Check memory usage (RAM should stay <32KB)
   - [ ] Test constraint handling (velocity/acceleration limits)

3. **Robustness testing** (4-6 hours)
   - [ ] Test with load variations (add inertia)
   - [ ] Test with friction variations
   - [ ] Test fast trajectory changes
   - [ ] Test solver failure handling (verify fallback to PID)
   - [ ] Thermal testing (extended operation)
   - [ ] Stress test (maximum speed profiles)

**Deliverable:** MPC achieves ≤1.7° RMS on hardware (better than or equal to PID)

---

## Testing Strategy

### Unit Tests (Host)
```bash
# Test MPC module in isolation
cargo test --features mpc mpc::tests

# Expected tests:
# - test_mpc_config_default()
# - test_mpc_state_conversion()
# - test_mpc_solve_simple_problem()
# - test_mpc_solver_convergence()
# - test_mpc_constraint_handling()
```

### Integration Tests (Renode)
```bash
# Build with MPC
cargo build --release --features mpc

# Run in Renode
./tools/renode/run_renode.sh

# Test scenarios:
# 1. MPC mode enabled, verify tracking
# 2. MPC mode disabled, verify PID still works
# 3. Mode switching during operation
# 4. MPC solver timeout → fallback to PID
# 5. Memory usage monitoring
```

### Hardware Tests
```bash
# System ID
python3 mpc/system_identification.py --port /dev/ttyACM0

# Performance test
python3 scripts/demos/demo_visualization.py --mpc-enabled

# Validation
python3 scripts/analysis/compare_trajectories.py --compare-mpc-vs-pid
```

---

## Success Criteria

### Must Have (Blocking)
- ✅ Firmware compiles with and without `--features mpc`
- ✅ PID controller works identically when MPC disabled
- ✅ MPC runs without crashes in Renode
- ✅ No memory overflows (RAM <32KB total)
- ✅ Solve time <500µs mean (for 1 kHz operation)

### Should Have (Goals)
- 🎯 RMS tracking error ≤1.7° (as good as PID)
- 🎯 Solver success rate >95%
- 🎯 Smooth mode switching (no discontinuities)
- 🎯 Graceful degradation on solver failures

### Nice to Have (Stretch)
- ⭐ RMS tracking error <1.0° (30% better than PID)
- ⭐ Solve time <200µs (could run at 2-5 kHz)
- ⭐ Online parameter adaptation
- ⭐ Multi-axis coordination

---

## Risk Mitigation

### Risk 1: RAM Overflow (High Impact)
**Problem:** OSQP workspace needs 15-20KB, only 32KB total RAM

**Mitigations:**
- Reduce horizon N (25 → 20 → 15)
- Use stack allocation instead of heap where possible
- Profile exact memory usage with `defmt::trace!`
- Add runtime assertions for stack overflow

**Fallback:** If RAM too tight, reduce N or disable MPC

---

### Risk 2: Solve Time Too Slow (Medium Impact)
**Problem:** Solver takes >500µs, can't run at 1 kHz

**Mitigations:**
- Tune OSQP parameters (reduce max_iter, increase tolerance)
- Reduce horizon N
- Use warm-start aggressively
- Profile with embedded timer to find bottlenecks

**Fallback:** Run MPC at 500 Hz or 250 Hz instead of 1 kHz

---

### Risk 3: No Performance Improvement (Medium Impact)
**Problem:** MPC tracking not better than PID

**Mitigations:**
- Re-tune cost weights (Q, R)
- Increase horizon for more preview
- Better system model (re-run system ID on hardware)
- Add feedforward terms

**Fallback:** Keep MPC as experimental feature, don't make it default

---

### Risk 4: Solver Instability (Low Impact)
**Problem:** Solver fails to converge, NaN outputs

**Mitigations:**
- Add regularization (increase R slightly)
- Check model conditioning (Lipschitz constant)
- Implement safety checks (bound outputs)
- Fallback to PID on any solver failure

**Fallback:** Disable MPC if failure rate >5%

---

## Implementation Checklist

### Phase 2b: FFI Integration
- [ ] Read OSQP C header (`osqp.h`)
- [ ] Create `extern "C"` declarations
- [ ] Write safe Rust wrappers
- [ ] Implement `solve()` method
- [ ] Add unit tests
- [ ] Test on host (`cargo test --features mpc`)

### Phase 2c: Firmware Integration
- [ ] Create 1 kHz MPC task
- [ ] Connect to position controller
- [ ] Add mode switching
- [ ] Test in Renode
- [ ] Profile solve times
- [ ] Check memory usage

### Phase 3: Validation
- [ ] Run system ID on hardware
- [ ] Update model parameters
- [ ] Regenerate embedded solver
- [ ] Flash to STM32
- [ ] Performance testing (track RMS error)
- [ ] Robustness testing (load/friction variations)
- [ ] Compare to PID baseline

---

## File Structure After Completion

```
mpc/
├── README.md                      # Overview, Phase 1-2a results
├── COMPLETION_PLAN.md             # This file
├── USAGE.md                       # When to use MPC vs PID
├── PHASE_2_PLAN.md                # Detailed Phase 2 architecture
├── system_identification.py       # System ID tool
├── mpc_controller.py              # Python prototype (reference)
├── generate_embedded_solver.py    # Code generation script
├── motor_model.json               # Identified parameters (update after hardware test)
├── embedded_mpc/                  # Generated C solver
│   ├── libemosqp.a                # Static library
│   └── inc/public/osqp.h          # C API
└── tests/                         # Test data
    └── hardware_validation.json   # Hardware test results

src/firmware/control/
└── mpc.rs                         # Rust MPC module (after Phase 2b)
    ├── FFI bindings (unsafe)
    ├── Safe wrappers
    ├── MPCController impl
    └── Unit tests
```

---

## Daily Schedule (Estimated)

### Day 1: FFI Bindings
- Morning: Study OSQP C API, write extern declarations
- Afternoon: Implement safe wrappers, basic solve()
- Evening: Unit tests, test on host

### Day 2: Solver Integration
- Morning: Complete solve() implementation, state conversion
- Afternoon: Error handling, warm-start, optimization
- Evening: Test with real problem data from Python prototype

### Day 3: Firmware Integration
- Morning: Create MPC async task, setup channels
- Afternoon: Connect to position controller, mode switching
- Evening: Test in Renode, debug issues

### Day 4: Renode Validation
- Morning: Profile solve times, optimize if needed
- Afternoon: Memory analysis, ensure <32KB total
- Evening: Robustness testing (failure scenarios)

### Day 5: Hardware System ID
- Morning: Flash firmware, run test sequences
- Afternoon: System ID on real data, update model
- Evening: Regenerate solver, rebuild firmware

### Day 6: Hardware Testing
- Morning: Performance testing (S-curve tracking)
- Afternoon: Robustness testing (load/friction)
- Evening: Final validation, documentation

---

## Decision Points

### After Phase 2b (FFI Complete)
**Question:** Does MPC solve() work correctly on host?

- ✅ YES → Proceed to Phase 2c (firmware integration)
- ❌ NO → Debug FFI bindings, check memory layout, verify C API usage

### After Phase 2c (Renode Testing)
**Question:** Is solve time <500µs? RAM usage OK?

- ✅ YES → Proceed to Phase 3 (hardware)
- ⚠️ SLOW (500-1000µs) → Reduce horizon, optimize, or accept 500 Hz
- ❌ FAILS (>1ms) → Consider code-gen option or abort MPC

### After Phase 3 (Hardware Validation)
**Question:** Is tracking error ≤1.7° (as good as PID)?

- ✅ YES → Merge MPC feature, document usage
- ⚠️ CLOSE (1.7-2.0°) → Tune weights, consider keeping as experimental
- ❌ NO IMPROVEMENT → Keep as research branch, don't merge

---

## Rollback Plan

If MPC feature fails at any phase:

1. **PID is unaffected** - Feature flag ensures PID works without MPC
2. **Remove MPC code** - Delete `--features mpc` section from Cargo.toml
3. **Document learnings** - Update README with "why MPC didn't work"
4. **Alternative paths:**
   - Try explicit MPC (pre-computed control law)
   - Use code-generated solver (FORCES, acados)
   - Improve PID with better feedforward
   - Accept 1.9° RMS as sufficient

**Key principle:** MPC is experimental. PID must always work.

---

## Next Immediate Action

**Start Phase 2b:**
1. Open `mpc/embedded_mpc/inc/public/osqp.h`
2. Identify key functions: `osqp_setup()`, `osqp_solve()`, `osqp_update_lin_cost()`
3. Write `extern "C"` declarations in `mpc.rs`
4. Create safe wrapper: `impl MPCController { pub fn solve() }`

**Command:**
```bash
# Read the C API
cat mpc/embedded_mpc/inc/public/osqp.h | grep "osqp_"

# Start implementing
code src/firmware/control/mpc.rs
```

---

*This plan will evolve as we discover issues. Update this file as progress is made.*
*Current Phase: 2b (FFI Integration) - NOT STARTED*
*Last Updated: 2025-10-10*
