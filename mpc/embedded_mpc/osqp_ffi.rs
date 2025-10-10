// Rust FFI bindings to OSQP embedded MPC solver
//
// This module provides safe Rust wrappers around the generated C solver.

use core::ffi::{c_float, c_int};

// Import generated C functions
extern "C" {
    fn osqp_setup(work: *mut OSQPWorkspace) -> c_int;
    fn osqp_solve(work: *mut OSQPWorkspace) -> c_int;
    fn osqp_update_lin_cost(work: *mut OSQPWorkspace, q_new: *const c_float) -> c_int;
    fn osqp_update_bounds(
        work: *mut OSQPWorkspace,
        l_new: *const c_float,
        u_new: *const c_float
    ) -> c_int;
    fn osqp_cleanup(work: *mut OSQPWorkspace);
}

// Workspace structure (must match C definition)
#[repr(C)]
struct OSQPWorkspace {
    // This is a placeholder - actual structure defined in generated code
    data: [u8; 1024],
}

/// Safe Rust wrapper for OSQP embedded solver
pub struct OSQPSolver {
    workspace: OSQPWorkspace,
}

impl OSQPSolver {
    pub fn new() -> Result<Self, &'static str> {
        let mut workspace = OSQPWorkspace { data: [0; 1024] };

        unsafe {
            let ret = osqp_setup(&mut workspace as *mut _);
            if ret == 0 {
                Ok(Self { workspace })
            } else {
                Err("OSQP setup failed")
            }
        }
    }

    pub fn solve(&mut self, q: &[f32], l: &[f32], u: &[f32]) -> Result<Vec<f32>, &'static str> {
        unsafe {
            // Update problem parameters
            osqp_update_lin_cost(&mut self.workspace as *mut _, q.as_ptr());
            osqp_update_bounds(&mut self.workspace as *mut _, l.as_ptr(), u.as_ptr());

            // Solve
            let ret = osqp_solve(&mut self.workspace as *mut _);
            if ret == 0 {
                // Extract solution (placeholder - actual implementation needed)
                Ok(vec![0.0; 25])
            } else {
                Err("OSQP solve failed")
            }
        }
    }
}

impl Drop for OSQPSolver {
    fn drop(&mut self) {
        unsafe {
            osqp_cleanup(&mut self.workspace as *mut _);
        }
    }
}
