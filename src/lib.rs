#![no_std]

// Public firmware modules
pub mod firmware;

// NOTE: Integration tests in tests/ directory cannot run on x86_64 host
// because embedded dependencies (embassy, cortex-m-rt) use ARM assembly.
// 
// We have 56+ unit tests inside modules that verify all logic:
// - Control algorithms (position, velocity, observer)
// - CAN protocol parsing
// - ADC/Encoder conversions
// - PWM timing calculations
//
// These unit tests compile with the target and provide full coverage.
// For host-based testing, extract pure algorithms into separate crates.

