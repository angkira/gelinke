#!/usr/bin/env python3
"""
OSQP Embedded Code Generation for MPC

This script generates malloc-free, library-free embedded C code
for the MPC optimization problem using OSQP code generation.

The generated solver can be compiled into the Rust firmware using FFI.

Usage:
    python3 generate_embedded_solver.py

Outputs:
    embedded_mpc/
        ├── include/
        │   ├── osqp.h
        │   ├── constants.h
        │   └── ...
        ├── src/
        │   ├── osqp.c
        │   └── ...
        └── README.txt

Author: Generated for MPC Phase 2
Date: 2025-10-10
"""

import numpy as np
import scipy.sparse as sp
import json
import os
import shutil


def load_system_model():
    """Load identified system model from Phase 1."""
    # Try current dir, then parent dir
    for path in ["motor_model.json", "../motor_model.json"]:
        if os.path.exists(path):
            with open(path, "r") as f:
                model = json.load(f)
            break
    else:
        raise FileNotFoundError("motor_model.json not found")

    A = np.array(model["A_matrix"])
    B = np.array(model["B_matrix"])
    dt = model["sampling_time"]

    return A, B, dt


def setup_mpc_problem(A, B, N=25, dt=0.001):
    """
    Setup MPC problem in OSQP format.

    State: x = [position, velocity, acceleration]
    Input: u = jerk

    Returns:
        P, q, A_osqp, l, u - QP problem matrices in OSQP format
    """
    n = 3  # State dimension
    m = 1  # Input dimension

    # Cost weights
    Q = np.diag([100.0, 10.0, 1.0])  # State cost
    R = np.array([[0.01]])  # Control cost

    # Constraints
    v_max = 2.0  # rad/s
    a_max = 5.0  # rad/s²
    j_max = 100.0  # rad/s³

    # Decision variables: [x_0, ..., x_N, u_0, ..., u_{N-1}]
    # Total: (N+1)*n + N*m = 3N + 3 + N = 4N + 3 variables

    n_vars = (N + 1) * n + N * m
    n_constraints = (N + 1) * n + N * m  # Dynamics + input bounds

    print(f"MPC Problem Size:")
    print(f"  Horizon: N = {N}")
    print(f"  Variables: {n_vars}")
    print(f"  Constraints: {n_constraints}")

    # ========================================
    # Cost function: (1/2) x'Px + q'x
    # ========================================

    # P matrix (block diagonal: Q for states, R for inputs)
    P_blocks = []

    # State cost blocks (Q for each time step)
    for k in range(N + 1):
        P_blocks.append(Q)

    # Control cost blocks (R for each time step)
    for k in range(N):
        P_blocks.append(R)

    P = sp.block_diag(P_blocks, format="csc")

    # q vector (all zeros since we'll update reference at runtime)
    # At runtime: q = -Q @ x_ref for tracking error
    q = np.zeros(n_vars)

    # ========================================
    # Constraints: l ≤ Ax ≤ u
    # ========================================

    # Constraint matrix structure:
    # 1. Initial condition: x_0 = x_init (set at runtime)
    # 2. Dynamics: x_{k+1} = A*x_k + B*u_k
    # 3. State bounds: -v_max ≤ vel ≤ v_max, etc.
    # 4. Input bounds: -j_max ≤ u ≤ j_max

    A_rows = []
    l_vec = []
    u_vec = []

    # --- Initial condition constraints ---
    # x_0 = x_init (set at runtime via l, u)
    A_init = sp.lil_matrix((n, n_vars))
    A_init[:n, :n] = sp.eye(n)
    A_rows.append(A_init)
    l_vec.extend([0.0, 0.0, 0.0])  # Will be updated at runtime
    u_vec.extend([0.0, 0.0, 0.0])

    # --- Dynamics constraints ---
    # x_{k+1} - A*x_k - B*u_k = 0
    for k in range(N):
        A_dyn = sp.lil_matrix((n, n_vars))

        # x_{k+1} term
        A_dyn[:n, (k + 1) * n : (k + 2) * n] = sp.eye(n)

        # -A*x_k term
        A_dyn[:n, k * n : (k + 1) * n] = -A

        # -B*u_k term
        A_dyn[:n, (N + 1) * n + k * m : (N + 1) * n + (k + 1) * m] = -B

        A_rows.append(A_dyn)
        l_vec.extend([0.0] * n)
        u_vec.extend([0.0] * n)

    # --- State bounds ---
    # For each time step: -v_max ≤ vel ≤ v_max, -a_max ≤ acc ≤ a_max
    for k in range(N + 1):
        A_bound = sp.lil_matrix((2, n_vars))

        # Velocity bound
        A_bound[0, k * n + 1] = 1.0
        l_vec.append(-v_max)
        u_vec.append(v_max)

        # Acceleration bound
        A_bound[1, k * n + 2] = 1.0
        l_vec.append(-a_max)
        u_vec.append(a_max)

        A_rows.append(A_bound)

    # --- Input bounds ---
    # -j_max ≤ u_k ≤ j_max
    for k in range(N):
        A_input = sp.lil_matrix((1, n_vars))
        A_input[0, (N + 1) * n + k] = 1.0
        A_rows.append(A_input)
        l_vec.append(-j_max)
        u_vec.append(j_max)

    # Stack all constraints
    A_osqp = sp.vstack(A_rows, format="csc")
    l = np.array(l_vec)
    u = np.array(u_vec)

    print(f"  P matrix: {P.shape}, nnz={P.nnz}")
    print(f"  A matrix: {A_osqp.shape}, nnz={A_osqp.nnz}")

    return P, q, A_osqp, l, u, N, n, m


def generate_embedded_code(P, q, A, l, u):
    """
    Generate embedded C code using OSQP code generation.

    Note: This requires osqp Python package with codegen support.
    If codegen is not available, this will create a placeholder.
    """
    try:
        import osqp

        # Setup OSQP problem
        prob = osqp.OSQP()
        prob.setup(
            P=P,
            q=q,
            A=A,
            l=l,
            u=u,
            verbose=False,
            eps_abs=1e-4,
            eps_rel=1e-4,
            max_iter=100,
        )

        # Try to generate code
        output_dir = "embedded_mpc"

        # Check if codegen is available
        if hasattr(prob, "codegen"):
            print(f"\n✓ Generating embedded C code to {output_dir}/")

            # Remove old directory if exists
            if os.path.exists(output_dir):
                shutil.rmtree(output_dir)

            # Generate code (API may vary by OSQP version)
            try:
                prob.codegen(
                    output_dir,
                    project_type="Makefile",
                    parameters="vectors",  # EMBEDDED_MODE=1 (can update q, l, u)
                    force_rewrite=True,
                )
            except TypeError:
                # Try simpler API for older/newer versions
                prob.codegen(output_dir, parameters="vectors")

            print(f"✓ Code generation complete!")
            print(f"\nGenerated files:")
            for root, dirs, files in os.walk(output_dir):
                for f in files:
                    rel_path = os.path.relpath(os.path.join(root, f), output_dir)
                    print(f"  {output_dir}/{rel_path}")

            # Create README
            with open(f"{output_dir}/README.txt", "w") as f:
                f.write("OSQP Embedded Solver for MPC\n")
                f.write("=" * 40 + "\n\n")
                f.write("Generated by: generate_embedded_solver.py\n")
                f.write("Date: 2025-10-10\n\n")
                f.write("This is a malloc-free, library-free embedded C solver.\n\n")
                f.write("To compile:\n")
                f.write("  cd embedded_mpc\n")
                f.write("  make\n\n")
                f.write("To integrate with Rust firmware:\n")
                f.write("  1. Copy files to src/firmware/mpc_solver/\n")
                f.write("  2. Create FFI bindings in build.rs\n")
                f.write("  3. Call from mpc.rs module\n")

            return True

        else:
            print("⚠️  OSQP codegen not available")
            print("    Install with: pip install osqp")
            print(
                "    Note: Some OSQP versions may not include codegen by default"
            )
            return False

    except ImportError:
        print("⚠️  osqp package not installed")
        print("    Install with: pip install osqp")
        return False


def create_rust_ffi_template(N, n, m):
    """Create a template for Rust FFI bindings to the generated C code."""

    template = f'''// Rust FFI bindings to OSQP embedded MPC solver
//
// This module provides safe Rust wrappers around the generated C solver.

use core::ffi::{{c_float, c_int}};

// Import generated C functions
extern "C" {{
    fn osqp_setup(work: *mut OSQPWorkspace) -> c_int;
    fn osqp_solve(work: *mut OSQPWorkspace) -> c_int;
    fn osqp_update_lin_cost(work: *mut OSQPWorkspace, q_new: *const c_float) -> c_int;
    fn osqp_update_bounds(
        work: *mut OSQPWorkspace,
        l_new: *const c_float,
        u_new: *const c_float
    ) -> c_int;
    fn osqp_cleanup(work: *mut OSQPWorkspace);
}}

// Workspace structure (must match C definition)
#[repr(C)]
struct OSQPWorkspace {{
    // This is a placeholder - actual structure defined in generated code
    data: [u8; 1024],
}}

/// Safe Rust wrapper for OSQP embedded solver
pub struct OSQPSolver {{
    workspace: OSQPWorkspace,
}}

impl OSQPSolver {{
    pub fn new() -> Result<Self, &'static str> {{
        let mut workspace = OSQPWorkspace {{ data: [0; 1024] }};

        unsafe {{
            let ret = osqp_setup(&mut workspace as *mut _);
            if ret == 0 {{
                Ok(Self {{ workspace }})
            }} else {{
                Err("OSQP setup failed")
            }}
        }}
    }}

    pub fn solve(&mut self, q: &[f32], l: &[f32], u: &[f32]) -> Result<Vec<f32>, &'static str> {{
        unsafe {{
            // Update problem parameters
            osqp_update_lin_cost(&mut self.workspace as *mut _, q.as_ptr());
            osqp_update_bounds(&mut self.workspace as *mut _, l.as_ptr(), u.as_ptr());

            // Solve
            let ret = osqp_solve(&mut self.workspace as *mut _);
            if ret == 0 {{
                // Extract solution (placeholder - actual implementation needed)
                Ok(vec![0.0; {N}])
            }} else {{
                Err("OSQP solve failed")
            }}
        }}
    }}
}}

impl Drop for OSQPSolver {{
    fn drop(&mut self) {{
        unsafe {{
            osqp_cleanup(&mut self.workspace as *mut _);
        }}
    }}
}}
'''

    return template


def main():
    print("=" * 60)
    print("OSQP EMBEDDED CODE GENERATION FOR MPC")
    print("=" * 60)

    # Load system model
    print("\n1. Loading system model from motor_model.json...")
    A, B, dt = load_system_model()
    print(f"   ✓ Model loaded (dt={dt}s)")

    # Setup MPC problem
    print("\n2. Setting up MPC QP problem...")
    N = 25  # Prediction horizon
    P, q, A_osqp, l, u, N, n, m = setup_mpc_problem(A, B, N=N, dt=0.001)
    print(f"   ✓ QP problem configured")

    # Generate embedded code
    print("\n3. Generating embedded C code...")
    success = generate_embedded_code(P, q, A_osqp, l, u)

    if success:
        # Create Rust FFI template
        print("\n4. Creating Rust FFI template...")
        ffi_code = create_rust_ffi_template(N, n, m)
        with open("embedded_mpc/osqp_ffi.rs", "w") as f:
            f.write(ffi_code)
        print(f"   ✓ Template created: embedded_mpc/osqp_ffi.rs")

        print("\n" + "=" * 60)
        print("✅ EMBEDDED CODE GENERATION COMPLETE")
        print("=" * 60)
        print("\nNext steps:")
        print("  1. cd embedded_mpc && make")
        print("  2. Review generated code")
        print("  3. Integrate with Rust firmware")
        print("  4. Test on target hardware")
    else:
        print("\n" + "=" * 60)
        print("⚠️  CODE GENERATION SKIPPED")
        print("=" * 60)
        print("\nAlternative approaches:")
        print("  - Use Rust OSQP bindings with std (desktop testing)")
        print("  - Implement simplified MPC without QP solver")
        print("  - Use commercial code generator (FORCES Pro, acados)")


if __name__ == "__main__":
    main()
