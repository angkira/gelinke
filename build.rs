// Build script for joint_firmware
//
// This script conditionally links the embedded OSQP solver when MPC feature is enabled.

fn main() {
    println!("cargo:rerun-if-changed=mpc/embedded_mpc/");

    // Only link OSQP library when MPC feature is enabled
    #[cfg(feature = "mpc")]
    {
        println!("cargo:rustc-link-search=native=mpc/embedded_mpc");
        println!("cargo:rustc-link-lib=static=emosqp");
        println!("cargo:warning=MPC feature enabled - linking embedded OSQP solver (+75KB)");
    }

    #[cfg(not(feature = "mpc"))]
    {
        println!("cargo:warning=MPC disabled (default). To enable: cargo build --features mpc");
    }
}
