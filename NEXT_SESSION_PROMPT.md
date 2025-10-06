# 🚀 iRPC v2.0 Development Session - Context & Instructions

## 📋 Project Context

### **What This Project Is**

**CLN17 v2.0 Joint Firmware** - FOC motor controller for robotic joints:
- **Hardware:** STM32G431CB microcontroller
- **Motor:** BLDC with FOC (Field-Oriented Control)
- **Communication:** iRPC protocol over CAN-FD
- **Framework:** Embassy async (embedded Rust)
- **Testing:** 95+ tests in Renode emulator

### **Current Status: EXCELLENT** ✅

```
✅ 100/100 Tests Complete (5% → 100% coverage)
✅ Production-ready testing infrastructure
✅ 3 Python mock peripherals (CAN, ADC, Encoder)
✅ Comprehensive documentation (2,800+ lines)
✅ iRPC v2.0 research COMPLETE (1,013 lines)
```

**Last Session Summary:**
- 10 git commits
- 8,600+ lines added
- Complete test suite activated
- TMC5160T research completed
- iRPC v2.0 architecture designed

---

## 🎯 Current Task: iRPC v2.0 Phase 1 - Foundation

### **Goal**

Implement foundational features for intelligent motion control:

1. ✅ **Motion Profiling** - Trapezoidal & S-curve trajectory generation
2. ✅ **Enhanced Protocol** - SetTargetV2 with acceleration/jerk control
3. ✅ **Trajectory Planner** - Real-time motion planning module
4. ✅ **Tests** - Comprehensive tests for new features
5. ✅ **Documentation** - Update protocol docs

**Expected Duration:** 2 weeks (60 hours)

### **Deliverables**

- `irpc_core` v2.0.0 with enhanced payloads
- Motion planner module in firmware
- Trajectory generation algorithms
- 20+ new tests for motion profiling
- Updated protocol documentation

---

## 📂 Project Structure

```
joint_firmware/
├── src/
│   └── firmware/
│       ├── tasks/          # Async tasks (CAN, FOC, etc.)
│       ├── drivers/        # Hardware drivers
│       ├── control/        # Control algorithms
│       │   ├── position.rs
│       │   ├── velocity.rs
│       │   └── motion_planner.rs  ← TO CREATE
│       ├── foc/           # FOC implementation
│       └── irpc_integration.rs  ← TO ENHANCE
│
├── renode/
│   ├── tests/             # Robot Framework tests (95 tests)
│   ├── peripherals/       # Python mocks (CAN, ADC, Encoder)
│   └── helpers/           # Test utilities
│
├── docs/
│   ├── IRPC_EVOLUTION_RESEARCH.md    ← REFERENCE THIS
│   ├── IRPC_V2_QUICK_SUMMARY.md      ← REFERENCE THIS
│   └── TESTING_SUITE.md
│
└── 100_TESTS_COMPLETE.md  ← Achievement summary
```

### **Key Files to Modify**

1. **iRPC Library** (`../../iRPC/` - sibling workspace):
   - `src/protocol.rs` - Add SetTargetPayloadV2, MotionProfile enum
   - `src/joint.rs` - Update Joint state machine

2. **Firmware**:
   - `src/firmware/control/motion_planner.rs` - NEW FILE
   - `src/firmware/irpc_integration.rs` - Handle v2 payloads
   - `src/firmware/tasks/foc.rs` - Use motion planner

3. **Tests**:
   - `renode/tests/motion_planning.robot` - NEW FILE (20 tests)
   - Update existing tests to use v2 protocol

---

## 🔧 Git Workflow Instructions

### **Branch Strategy**

**CRITICAL:** Each feature gets its own branch!

```bash
# Branch naming convention:
feature/irpc-v2-<feature-name>

# Examples:
feature/irpc-v2-motion-profiling
feature/irpc-v2-enhanced-protocol
feature/irpc-v2-trajectory-planner
feature/irpc-v2-tests
```

### **Workflow for Each Feature**

```bash
# 1. Create feature branch from main
git checkout main
git pull
git checkout -b feature/irpc-v2-motion-profiling

# 2. Work on feature
# - Make incremental commits
# - Each commit should be atomic and functional

# 3. Commit messages format:
git commit -m "feat(motion): Add trapezoidal motion profile generator

- Implement position, velocity, acceleration calculation
- Add time-optimal trajectory planning
- Include acceleration/deceleration phases
- Unit tests for profile generation

Refs: IRPC_EVOLUTION_RESEARCH.md Phase 1"

# 4. Push feature branch
git push origin feature/irpc-v2-motion-profiling

# 5. When feature complete, merge to main
git checkout main
git merge --no-ff feature/irpc-v2-motion-profiling
git push origin main

# 6. Delete feature branch (optional)
git branch -d feature/irpc-v2-motion-profiling
```

### **Commit Message Format**

```
<type>(<scope>): <subject>

<body>

<footer>
```

**Types:**
- `feat`: New feature
- `fix`: Bug fix
- `refactor`: Code restructuring
- `test`: Adding tests
- `docs`: Documentation
- `perf`: Performance improvement
- `style`: Code style changes

**Scopes:**
- `motion`: Motion planning/profiling
- `protocol`: iRPC protocol changes
- `control`: Control algorithms
- `test`: Testing infrastructure
- `docs`: Documentation

**Example:**
```
feat(motion): Implement S-curve motion profile

- Add jerk-limited trajectory generation
- Calculate smooth acceleration curves
- Minimize mechanical vibrations
- Optimize motion time under jerk constraints

Performance:
- 40% faster motion vs trapezoidal
- 60% vibration reduction
- Works with all motor sizes

Tests: Added 8 tests for S-curve generation
Refs: IRPC_EVOLUTION_RESEARCH.md Section 5.1
```

---

## 📚 Technical References

### **Research Documents**

1. **`docs/IRPC_EVOLUTION_RESEARCH.md`** (839 lines)
   - Section 3: TMC5160T Features
   - Section 4.1: Enhanced Motion Commands
   - Section 5: Implementation Roadmap Phase 1

2. **`docs/IRPC_V2_QUICK_SUMMARY.md`** (174 lines)
   - Quick overview of features
   - Impact metrics

### **Current iRPC Protocol**

**Location:** `../../iRPC/src/protocol.rs`

**Current SetTarget:**
```rust
pub struct SetTargetPayload {
    pub target_angle: f32,      // Degrees
    pub velocity_limit: f32,    // Degrees/second
}
```

**Target SetTargetV2:**
```rust
pub struct SetTargetPayloadV2 {
    pub target_angle: f32,           // Degrees
    pub max_velocity: f32,           // Degrees/second
    pub target_velocity: f32,        // Final velocity (for fly-by)
    pub max_acceleration: f32,       // Degrees/second²
    pub max_deceleration: f32,       // Degrees/second²
    pub max_jerk: Option<f32>,       // Degrees/second³
    pub profile: MotionProfile,
    pub max_current: Option<f32>,    // Amperes
    pub max_temperature: Option<f32>,// Celsius
}

pub enum MotionProfile {
    Trapezoidal,    // Constant accel/decel
    SCurve,         // Jerk-limited
    Adaptive,       // Load-adaptive
}
```

### **Motion Planning Algorithms**

**Trapezoidal Profile:**
```
Velocity
   ^
   |     ___________
   |    /           \
   |   /             \
   |  /               \
   | /                 \
   |/                   \___
   +-----------------------> Time
   
   Accel | Const Vel | Decel
```

**S-Curve Profile:**
```
Velocity
   ^
   |      _______
   |    /         \
   |   /           \
   |  /             \
   | /               \
   |/                 \___
   +-----------------------> Time
   
   Smooth transitions (jerk-limited)
```

**Key Equations:**
```
Trapezoidal:
- t_accel = v_max / a_max
- t_const = (distance - v_max²/a_max) / v_max
- t_decel = v_max / a_max

S-Curve:
- t_jerk_up = a_max / j_max
- t_accel_const = (v_max - a_max²/j_max) / a_max
- Similar for deceleration
```

---

## 🎯 Phase 1 Detailed Tasks

### **Task 1: Motion Profile Generators** (20 hours)

**File:** `src/firmware/control/motion_planner.rs`

**What to implement:**
```rust
pub struct MotionPlanner {
    // Configuration
    config: MotionConfig,
    // Current trajectory
    current_trajectory: Option<Trajectory>,
}

pub struct MotionConfig {
    pub max_velocity: f32,
    pub max_acceleration: f32,
    pub max_jerk: f32,
}

pub struct Trajectory {
    pub profile_type: MotionProfile,
    pub waypoints: Vec<TrajectoryPoint>,
    pub total_time: f32,
}

pub struct TrajectoryPoint {
    pub time: f32,
    pub position: f32,
    pub velocity: f32,
    pub acceleration: f32,
}

impl MotionPlanner {
    pub fn plan_trapezoidal(&self, 
        start: f32, 
        end: f32, 
        max_vel: f32, 
        max_accel: f32
    ) -> Trajectory;
    
    pub fn plan_scurve(&self, 
        start: f32, 
        end: f32, 
        max_vel: f32, 
        max_accel: f32, 
        max_jerk: f32
    ) -> Trajectory;
    
    pub fn interpolate(&self, trajectory: &Trajectory, time: f32) -> TrajectoryPoint;
}
```

**Tests to write:**
- Trapezoidal: acceleration phase correctness
- Trapezoidal: constant velocity phase
- Trapezoidal: deceleration phase
- S-curve: jerk limiting
- S-curve: smooth transitions
- Time-optimal planning
- Edge cases (short moves, velocity limits)

### **Task 2: Protocol Enhancement** (15 hours)

**File:** `../../iRPC/src/protocol.rs`

**What to add:**
```rust
// Add to Payload enum
pub enum Payload {
    // ... existing variants
    
    // NEW for v2.0
    SetTargetV2(SetTargetPayloadV2),
}

// Add new struct
pub struct SetTargetPayloadV2 {
    // ... fields from research doc
}

// Add enum
pub enum MotionProfile {
    Trapezoidal,
    SCurve,
    Adaptive,
}
```

**Maintain backward compatibility:**
- Keep existing `SetTarget` for v1.0 clients
- Add feature flag `irpc_v2` for new payloads

### **Task 3: Firmware Integration** (15 hours)

**File:** `src/firmware/irpc_integration.rs`

**What to modify:**
```rust
impl JointFocBridge {
    pub fn handle_message(&mut self, msg: &Message) -> Option<Message> {
        match &msg.payload {
            Payload::SetTarget(target) => {
                // v1.0 - simple target
                self.apply_target_v1(target);
            }
            Payload::SetTargetV2(target) => {
                // v2.0 - motion planning
                self.apply_target_v2(target);
            }
            // ... rest
        }
    }
    
    fn apply_target_v2(&mut self, target: &SetTargetPayloadV2) {
        // Generate trajectory
        let trajectory = self.motion_planner.plan(
            self.current_position(),
            target.target_angle,
            target.max_velocity,
            target.max_acceleration,
            target.profile,
        );
        
        // Store for FOC loop
        self.current_trajectory = Some(trajectory);
    }
}
```

### **Task 4: Tests** (10 hours)

**File:** `renode/tests/motion_planning.robot`

**Tests to write:**
```robot
*** Test Cases ***

Should Generate Trapezoidal Profile
    [Documentation]         Generate trapezoidal velocity profile
    [Tags]                  motion  trapezoidal
    
    # Test setup
    # Generate profile: 0° → 90° @ 100°/s, 500°/s²
    # Verify waypoints
    # Check acceleration/constant/deceleration phases

Should Generate S-Curve Profile
    [Documentation]         Generate jerk-limited S-curve profile
    [Tags]                  motion  scurve
    
    # Test setup
    # Generate profile with jerk limit
    # Verify smooth transitions
    # Check jerk constraints

Should Handle Short Moves
    [Documentation]         Motion too short for constant velocity
    [Tags]                  motion  edge-case
    
    # Test triangular profile (no const vel phase)

Should Respect Velocity Limits
    [Documentation]         Velocity should not exceed max
    [Tags]                  motion  limits
    
    # Verify velocity clamping

Should Optimize Motion Time
    [Documentation]         Minimize time under constraints
    [Tags]                  motion  optimization
    
    # Compare with theoretical minimum

Should Track Trajectory In FOC Loop
    [Documentation]         FOC follows generated trajectory
    [Tags]                  motion  foc  integration
    
    # End-to-end test with FOC loop
    # Verify position tracking

... (14 more tests)
```

---

## 🚨 Important Guidelines

### **Development Principles**

1. ✅ **Clean Code** - SOLID, DRY, KISS principles
2. ✅ **Test First** - Write tests before/with implementation
3. ✅ **Incremental** - Small, atomic commits
4. ✅ **Documentation** - Comment complex algorithms
5. ✅ **Performance** - Profile critical paths
6. ✅ **Safety** - Validate all inputs, handle errors

### **Code Style**

```rust
// ✅ GOOD: Descriptive, documented
/// Calculate trapezoidal motion profile waypoints.
///
/// Generates a time-optimal trajectory with constant acceleration
/// and deceleration phases.
///
/// # Arguments
/// * `start` - Initial position (degrees)
/// * `end` - Target position (degrees)
/// * `max_vel` - Maximum velocity (degrees/second)
/// * `max_accel` - Maximum acceleration (degrees/second²)
///
/// # Returns
/// Trajectory with position/velocity/acceleration at each timestep
pub fn plan_trapezoidal(
    start: f32,
    end: f32,
    max_vel: f32,
    max_accel: f32,
) -> Trajectory {
    // Implementation
}

// ❌ BAD: No docs, unclear
pub fn plan(s: f32, e: f32, v: f32, a: f32) -> Vec<f32> {
    // ...
}
```

### **Error Handling**

```rust
// ✅ GOOD: Explicit error types
pub enum MotionPlanningError {
    InvalidParameters,
    InfeasibleTrajectory,
    NumericInstability,
}

pub fn plan_trajectory(...) -> Result<Trajectory, MotionPlanningError> {
    if max_vel <= 0.0 {
        return Err(MotionPlanningError::InvalidParameters);
    }
    // ...
}

// ❌ BAD: Panics in production code
pub fn plan_trajectory(...) -> Trajectory {
    assert!(max_vel > 0.0);  // Will panic!
    // ...
}
```

### **Testing**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_trapezoidal_phases() {
        let planner = MotionPlanner::new(Default::default());
        let traj = planner.plan_trapezoidal(0.0, 90.0, 100.0, 500.0);
        
        // Verify acceleration phase
        let t_accel = traj.waypoints.iter()
            .take_while(|p| p.velocity < 100.0)
            .count();
        assert!(t_accel > 0);
        
        // Verify constant velocity phase
        let const_vel = traj.waypoints.iter()
            .filter(|p| (p.velocity - 100.0).abs() < 0.1)
            .count();
        assert!(const_vel > 0);
        
        // ... more assertions
    }
}
```

---

## 📊 Success Criteria

### **Phase 1 Complete When:**

✅ **Functionality:**
- [ ] Trapezoidal profile generator working
- [ ] S-curve profile generator working
- [ ] Motion planner integrated in firmware
- [ ] SetTargetV2 protocol implemented
- [ ] FOC loop follows trajectories

✅ **Quality:**
- [ ] 20+ tests passing (motion_planning.robot)
- [ ] All unit tests passing
- [ ] No compiler warnings
- [ ] Code coverage > 80%

✅ **Performance:**
- [ ] Motion planning < 1 ms
- [ ] FOC loop still runs at 10 kHz
- [ ] No memory leaks
- [ ] Trajectory interpolation < 10 µs

✅ **Documentation:**
- [ ] Protocol documentation updated
- [ ] API docs complete
- [ ] Usage examples provided
- [ ] Migration guide written

---

## 🎓 Helpful Resources

### **Motion Planning Theory**

- "Robot Modeling and Control" - Spong et al.
- "Planning Algorithms" - Steven LaValle
- Jerk-limited trajectories: [Wikipedia](https://en.wikipedia.org/wiki/Jerk_(physics))

### **Rust Embedded**

- Embassy async framework docs
- no_std Rust patterns
- fixed-point math library

### **Testing**

- Robot Framework documentation
- Renode emulator guide
- Python peripherals API

---

## 💬 Communication Style

**When working with me:**

1. ✅ **Start with planning** - Outline approach before coding
2. ✅ **Show your work** - Explain design decisions
3. ✅ **Incremental progress** - Small commits, regular updates
4. ✅ **Ask questions** - Clarify requirements early
5. ✅ **Test thoroughly** - Don't skip tests
6. ✅ **Document decisions** - Update docs as you go

**I prefer:**
- 📊 Code over talk (show, don't tell)
- 🎯 Direct solutions over explanations
- ⚡ Fast iteration over perfect first attempt
- 🧪 Tests as proof of correctness

---

## 🚀 Let's Start!

**Your first message should be:**

1. ✅ Confirm you understand the task
2. ✅ Outline your approach for Task 1 (Motion Planner)
3. ✅ Create feature branch
4. ✅ Start coding!

**Example:**
```
Ready to implement Phase 1: Foundation! 🚀

Approach:
1. Create feature/irpc-v2-motion-profiling branch
2. Implement MotionPlanner struct with trapezoidal algorithm
3. Add unit tests
4. Integrate with Position controller
5. Test in Renode

Starting with motion_planner.rs...
```

---

## 📎 Quick Commands

```bash
# Build firmware
cargo build --release --features renode-mock

# Run tests
cargo test
renode-test renode/tests/

# Check code
cargo clippy
cargo fmt --check

# Documentation
cargo doc --open

# Git workflow
git checkout -b feature/irpc-v2-<name>
git commit -m "feat(motion): <description>"
git push origin feature/irpc-v2-<name>
```

---

**GO! ПОГНАЛИ! 🚀💪**

**Remember:** Each feature = new branch. Clean commits. Test everything. Have fun! 😎
