# MPC Usage Guide

## MPC is Optional

**By default, the firmware uses the proven PID controller** (1.903° RMS tracking accuracy). MPC is an experimental feature for achieving <1° RMS tracking at the cost of additional resources.

## Building Without MPC (Default)

```bash
# Standard build - no MPC, uses PID controller
cargo build --release

# Binary size: ~normal
# RAM usage: ~normal
# Control: PID cascade (position P + velocity PID)
```

## Building With MPC (Experimental)

```bash
# Enable MPC feature
cargo build --release --features mpc

# Binary size: +75 KB (OSQP solver)
# RAM usage: +15-20 KB (optimization workspace)
# Control: 1 kHz MPC outer loop + 10 kHz PID inner loop
```

## Feature Comparison

| Feature | PID (Default) | MPC (Experimental) |
|---------|---------------|-------------------|
| **RMS Error** | 1.903° | 1.655° (13% better) |
| **Flash** | Baseline | +75 KB |
| **RAM** | Baseline | +15-20 KB |
| **CPU** | 10 µs @ 10 kHz | 500 µs @ 1 kHz |
| **Maturity** | ✅ Proven | ⚠️ Experimental |
| **Hardware Support** | All | STM32G4+ (needs 32KB+ RAM) |

## When to Use MPC

**Use PID (default) if:**
- 1.9° tracking is good enough
- Flash/RAM is limited
- You need proven, production-ready control
- Simplicity is important

**Use MPC if:**
- You need <1° RMS tracking
- You have STM32G4/H7 with >32KB RAM
- You're willing to test and tune an experimental feature
- You need constraint handling (velocity/acceleration limits)

## Runtime Behavior

### With MPC Disabled (Default)
- Position controller runs at 10 kHz
- Pure PID cascade control
- No optimization solver
- Fast, deterministic

### With MPC Enabled
- MPC outer loop: 1 kHz (generates position/velocity setpoints)
- PID inner loop: 10 kHz (tracks MPC setpoints)
- OSQP solver: ~500 µs per solve
- Predictive, constraint-aware

## Checking Feature Status

```rust
// In your code, check if MPC is available:
#[cfg(feature = "mpc")]
{
    // MPC-specific code
    use crate::firmware::control::mpc::MPCController;
}

#[cfg(not(feature = "mpc"))]
{
    // PID-only code (default)
}
```

## Cargo.toml Feature Flags

```toml
[features]
default = []  # No MPC by default
mpc = []      # Enable MPC optimization
```

## Testing

```bash
# Test without MPC (should be default behavior)
cargo test

# Test with MPC
cargo test --features mpc

# Check binary size difference
cargo size --release
cargo size --release --features mpc
```

## Recommendation

**Start with the default PID controller.** It's proven, efficient, and provides excellent tracking (1.9° RMS).

**Only enable MPC if:**
1. You've validated PID performance on your hardware
2. You need the extra 13% tracking improvement
3. You have the flash/RAM budget
4. You're willing to tune and test MPC parameters

The PID controller will **not** be broken or affected by MPC code - they are completely independent.
