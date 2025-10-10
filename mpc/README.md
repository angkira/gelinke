# Model Predictive Control (MPC) Implementation

**Phase 1:** ✅ COMPLETE - Python prototype, 13% improvement over PID
**Phase 2a:** ✅ COMPLETE - Embedded C code generated (75KB, malloc-free)
**Date:** 2025-10-10
**Current:** Phase 2b - Rust FFI integration

---

## Phase 2a Results (Embedded Code Generation)

### What Was Accomplished

✅ **Generated malloc-free C solver** using OSQP code generation
✅ **Compiled successfully** - 75KB static library
✅ **Verified functionality** - solver runs without errors
✅ **Created Rust FFI template** for integration

### Generated Artifacts

```
mpc/embedded_mpc/
├── libemosqp.a          # 75KB static library (compiled)
├── emosqp               # Test executable
├── inc/                 # Header files
│   ├── public/osqp.h    # Main API
│   └── private/...      # Internal headers
├── src/                 # Source files
│   ├── osqp_api.c       # Main solver interface
│   ├── qdldl.c          # Matrix factorization
│   └── ...
├── osqp_ffi.rs          # Rust FFI template
└── README.txt           # Integration instructions
```

### Key Specifications

| Property | Value |
|----------|-------|
| **Library Size** | 75 KB |
| **Memory Mode** | Static (no malloc) |
| **Dependencies** | None (library-free) |
| **Problem Size** | N=25, 103 variables, 155 constraints |
| **Sparsity** | P: 259 nnz, A: 380 nnz |

### Code Generation

```bash
# Generate embedded solver from Python prototype
python3 mpc/generate_embedded_solver.py

# Compile C library
cd mpc/embedded_mpc && make

# Test
./emosqp
```

### Next Steps for Phase 2b

1. ✅ Create build.rs for Rust linking
2. 🔄 Write safe Rust FFI wrappers
3. ⏳ Integrate with firmware control loop
4. ⏳ Test on target hardware

---

## Quick Start

### 1. System Identification

```bash
# Run system identification on test data
python3 mpc/system_identification.py

# This generates:
# - motor_model.json (identified parameters)
# - validation_plot.png (model validation)
# - test_sequences.json (test procedures for hardware)
```

### 2. Test MPC Controller

```bash
# Run MPC tracking simulation
python3 mpc/mpc_controller.py

# This generates:
# - mpc_tracking_results.png (tracking performance)
# - Console output with RMS error and solve times
```

---

## Performance Results (Phase 1)

### MPC vs PID Comparison

| Metric | PID (Phase 3) | MPC (Phase 1) | Improvement |
|--------|---------------|---------------|-------------|
| **RMS Error** | 1.903° | **1.655°** | **+13%** ✅ |
| **Max Error** | 3.568° | **2.684°** | **+25%** ✅ |
| **Overshoot** | 3.5% | **~2%** | **+43%** ✅ |
| **Computation** | 10 µs | **3844 µs** | **384x slower** ⚠️ |

### Key Findings:

✅ **MPC achieves better tracking** (13% RMS improvement)
✅ **Smoother control** (reduced max error by 25%)
✅ **100% solver success rate** (robust optimization)

⚠️ **Computation time too high** for 10 kHz (need <100 µs, currently 3.8ms)
⚠️ **Not yet <1° RMS target** (1.655° vs 1.0° goal)

---

## Architecture

### State-Space Model

```
State: x = [position, velocity, acceleration]
Input: u = jerk

Dynamics: x[k+1] = A*x[k] + B*u[k]

where A, B are identified from system tests
```

### MPC Formulation

```
minimize: Σ(Q*(x-x_ref)² + R*u²)

subject to:
  x[k+1] = A*x[k] + B*u[k]  (dynamics)
  |velocity| ≤ 2.0 rad/s
  |accel| ≤ 5.0 rad/s²
  |jerk| ≤ 100 rad/s³
```

**Parameters:**
- Horizon N = 50 steps (5ms lookahead)
- Q = diag([100, 10, 1]) - State weights (pos, vel, acc)
- R = 0.01 - Control effort weight
- Solver: OSQP (Quadratic Programming)

---

## Files Structure

```
mpc/
├── system_identification.py  - System ID from test data
├── mpc_controller.py          - MPC controller (Python/CVXPY)
├── README.md                  - This file
└── motor_model.json          - Identified system model

Generated outputs:
├── validation_plot.png        - System ID validation
├── mpc_tracking_results.png   - MPC performance
└── test_sequences.json        - Hardware test procedures
```

---

## Next Steps (Phase 2: C Implementation)

### Option A: Optimize Python First (1-2 days)

**Goal:** Reduce solve time to <100 µs

1. Reduce horizon N (50 → 30) - reduces QP size
2. Use sparse matrices explicitly
3. Tune OSQP settings (lower accuracy, more iterations)
4. Warm-start more aggressively

**Expected:** 1-2ms solve time (10x faster, but still not 10 kHz ready)

### Option B: Port to C with OSQP (Week 2 of plan)

**Goal:** Embedded implementation at 5-10 kHz

1. Install OSQP C library
2. Implement MPC in C (see `MPC_IMPLEMENTATION_PLAN.md`)
3. Cross-compile for ARM/STM32
4. Integrate with firmware

**Expected:** 50-200 µs solve time (with optimization)

### Option C: Accept Current Performance (Recommended)

**MPC at 1 kHz instead of 10 kHz:**
- 1ms solve time → 1 kHz outer loop
- 10 kHz inner PID for current control
- MPC generates position/velocity setpoints
- Still get ~10% RMS improvement
- Much simpler to implement

---

## Installation

### Python Dependencies

```bash
pip install cvxpy osqp numpy scipy matplotlib
```

### Hardware Requirements (for C implementation)

- STM32F7/H7 (or equivalent, 200+ MHz)
- 100+ KB RAM for QP solver
- 100+ KB Flash for OSQP library

---

## Usage Example

```python
from mpc_controller import MPCController
import numpy as np

# Load system model
import json
with open('motor_model.json') as f:
    model = json.load(f)

A = np.array(model['A_matrix'])
B = np.array(model['B_matrix'])

# Create MPC controller
mpc = MPCController(
    A=A, B=B,
    N=50,           # Prediction horizon
    dt=0.0001,      # 10 kHz sampling
    Q=np.array([100, 10, 1]),  # State costs
    R=0.01,         # Control cost
)

# At each control cycle:
x_current = np.array([pos, vel, acc])
x_ref_horizon = generate_reference_trajectory()  # (3, N+1)

jerk_cmd, info = mpc.solve(x_current, x_ref_horizon)

# Apply jerk command
acc += jerk_cmd * dt
vel += acc * dt
pos += vel * dt
```

---

## Troubleshooting

### Slow solve times (>10ms)

- **Reduce horizon:** N=50 → N=30
- **Simplify costs:** Use diagonal Q matrix
- **Tune OSQP:** Increase `eps_abs`, `eps_rel` to 1e-3

### Poor tracking (high RMS error)

- **Increase Q weights:** Q=[100,10,1] → Q=[200,20,2]
- **Decrease R weight:** R=0.01 → R=0.001
- **Increase horizon:** N=50 → N=80

### Solver failures

- **Check model validity:** Ensure A, B are correct
- **Loosen constraints:** Increase v_max, a_max limits
- **Add regularization:** Increase R slightly

---

## Performance Tuning Guide

### Cost Weights (Q, R)

**Q = [Q_pos, Q_vel, Q_acc]:**
- ↑ Q_pos → More aggressive position tracking
- ↑ Q_vel → Smoother velocity profile
- ↑ Q_acc → Smoother acceleration

**R (control effort):**
- ↓ R → More aggressive control (faster response)
- ↑ R → Gentler control (smoother, but slower)

**Rule of thumb:**
- Q_pos / Q_vel ≈ 10 (position 10x more important)
- Q_vel / Q_acc ≈ 10 (velocity 10x more important)
- R ≈ 0.001 - 0.1 (start with 0.01)

### Horizon Length (N)

- **Shorter (N=20-30):** Faster solve, myopic control
- **Longer (N=60-100):** Slower solve, better preview

**Optimal:** N ≈ settling_time / dt
- For 0.5s settling @ 10kHz: N ≈ 5000 (too large!)
- Practical: N = 30-60 (3-6ms lookahead)

---

## Known Limitations

1. **Computation time:** 3.8ms mean (38x too slow for 10 kHz)
   - Mitigations: Reduce N, optimize code, use C implementation
   - Alternative: Run MPC at 1 kHz (still useful)

2. **Not <1° RMS yet:** 1.655° (need 0.65° more improvement)
   - Mitigations: Better model, tune weights, longer horizon
   - May need model-based feedforward on top of MPC

3. **Python only:** Not embedded-ready
   - Solution: Phase 2 C implementation (Week 2 of plan)

4. **No load adaptation:** Fixed model
   - Solution: Online system ID or adaptive MPC (advanced)

---

## Conclusion

**Phase 1 Status:** ✅ **COMPLETE**

MPC prototype demonstrates **13% improvement** over PID in simulation.
Computation time is high but can be optimized in C implementation.

**Recommendation:**
- If you need <1° RMS → Continue to Phase 2 (C implementation + optimization)
- If 1.7° RMS is acceptable → Deploy optimized PID (simpler, good enough)
- If resources limited → Use MPC at 1 kHz (10x slower but still effective)

**Next milestone:** Phase 2 - C implementation targeting <100 µs solve time

---

*Last updated: 2025-10-08*
*Python MPC prototype: 1.655° RMS, 13% better than PID*
*Ready for Phase 2: C implementation*
