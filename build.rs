// Build script for joint_firmware
//
// This script compiles the embedded OSQP solver and links it with the firmware.

fn main() {
    // Configure linker for embedded OSQP library
    println!("cargo:rerun-if-changed=mpc/embedded_mpc/");

    // For now, OSQP integration is compile-time optional
    // The library exists but isn't linked yet to avoid bloating firmware
    // Uncomment below when ready to integrate:

    // println!("cargo:rustc-link-search=native=mpc/embedded_mpc");
    // println!("cargo:rustc-link-lib=static=emosqp");

    println!("cargo:warning=MPC embedded solver available in mpc/embedded_mpc/");
    println!("cargo:warning=To enable MPC, uncomment linker flags in build.rs");
}
