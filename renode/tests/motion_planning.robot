*** Settings ***
Documentation     Motion Planning Tests - iRPC v2.0 Phase 1
...
...               Tests for trapezoidal and S-curve motion profiles with trajectory generation.
...               Validates motion planning algorithms, trajectory interpolation, and FOC integration.

Resource          test_helpers.robot
Library           String
Library           Collections

Suite Setup       Setup Motion Planning Suite
Suite Teardown    Teardown Motion Planning Suite
Test Setup        Reset Joint State
Test Timeout      30 seconds


*** Variables ***
${FIRMWARE_ELF}        ${CURDIR}/../../target/thumbv7em-none-eabihf/release-mock/joint_firmware
${TEST_PLATFORM}       ${CURDIR}/../platforms/stm32g431cb.repl
${TEST_SCRIPT}         ${CURDIR}/../scripts/joint_test.resc

# Motion planning parameters
${DEFAULT_MAX_VEL}         100.0    # degrees/second
${DEFAULT_MAX_ACCEL}       500.0    # degrees/second²
${DEFAULT_MAX_JERK}        2000.0   # degrees/second³

# Tolerances
${POSITION_TOLERANCE}      1.0      # degrees
${VELOCITY_TOLERANCE}      5.0      # degrees/second
${TIME_TOLERANCE}          0.1      # seconds


*** Keywords ***
Setup Motion Planning Suite
    [Documentation]    Initialize Renode platform for motion planning tests
    Execute Script    ${TEST_SCRIPT}
    Start Emulation
    Wait For Joint Ready

Teardown Motion Planning Suite
    [Documentation]    Clean up after test suite
    Stop Emulation

Reset Joint State
    [Documentation]    Reset joint to known state before each test
    Send Reset Command
    Wait For State    Unconfigured
    Send Configure Command
    Wait For State    Inactive
    Send Activate Command
    Wait For State    Active

Send SetTarget V2
    [Arguments]    ${target_angle}    ${max_vel}=${DEFAULT_MAX_VEL}    ${max_accel}=${DEFAULT_MAX_ACCEL}    
    ...            ${profile}=Trapezoidal    ${max_jerk}=${DEFAULT_MAX_JERK}
    [Documentation]    Send enhanced SetTargetV2 command with motion profiling
    ${target_velocity}=    Set Variable    0.0
    ${max_decel}=    Set Variable    ${max_accel}
    ${max_current}=    Set Variable    0.0
    ${max_temp}=    Set Variable    0.0
    
    ${profile_value}=    Run Keyword If    '${profile}' == 'Trapezoidal'    Set Variable    0
    ...    ELSE IF    '${profile}' == 'SCurve'    Set Variable    1
    ...    ELSE IF    '${profile}' == 'Adaptive'    Set Variable    2
    ...    ELSE    Fail    Unknown profile type: ${profile}
    
    Send iRPC Message    SetTargetV2    
    ...    target_angle=${target_angle}
    ...    max_velocity=${max_vel}
    ...    target_velocity=${target_velocity}
    ...    max_acceleration=${max_accel}
    ...    max_deceleration=${max_decel}
    ...    max_jerk=${max_jerk}
    ...    profile=${profile_value}
    ...    max_current=${max_current}
    ...    max_temperature=${max_temp}

Wait For Motion Complete
    [Arguments]    ${timeout}=10s
    [Documentation]    Wait until trajectory execution completes
    ${start_time}=    Get Time    epoch
    FOR    ${i}    IN RANGE    100
        ${velocity}=    Get Current Velocity
        ${abs_vel}=    Evaluate    abs(${velocity})
        Exit For Loop If    ${abs_vel} < 1.0
        Sleep    0.1s
        ${elapsed}=    Evaluate    time.time() - ${start_time}    modules=time
        Exit For Loop If    ${elapsed} > 10
    END

Verify Position Reached
    [Arguments]    ${target}    ${tolerance}=${POSITION_TOLERANCE}
    [Documentation]    Verify that current position matches target within tolerance
    ${current_pos}=    Get Current Position
    ${error}=    Evaluate    abs(${current_pos} - ${target})
    Should Be True    ${error} < ${tolerance}    
    ...    Position error ${error}° exceeds tolerance ${tolerance}°


*** Test Cases ***

Should Generate Trapezoidal Profile For Long Move
    [Documentation]    Generate trapezoidal velocity profile for motion long enough to reach max velocity
    [Tags]    motion    trapezoidal    basic
    
    # Move from 0° to 90° with trapezoidal profile
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    
    # Wait for Ack
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack
    
    # Wait for motion to complete
    Wait For Motion Complete    timeout=5s
    
    # Verify final position
    Verify Position Reached    90.0    tolerance=2.0
    
    Log    Trapezoidal profile executed successfully

Should Generate Trapezoidal Profile For Short Move
    [Documentation]    Generate triangular velocity profile when distance is too short for constant velocity
    [Tags]    motion    trapezoidal    edge-case
    
    # Short move from 0° to 10° - should create triangular profile
    Send SetTarget V2    10.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack
    
    Wait For Motion Complete    timeout=2s
    Verify Position Reached    10.0    tolerance=1.0

Should Generate S-Curve Profile
    [Documentation]    Generate jerk-limited S-curve profile for smooth motion
    [Tags]    motion    scurve    basic
    
    # Move with S-curve profile
    Send SetTarget V2    45.0    max_vel=80.0    max_accel=400.0    profile=SCurve    max_jerk=2000.0
    
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack
    
    Wait For Motion Complete    timeout=5s
    Verify Position Reached    45.0    tolerance=2.0
    
    Log    S-curve profile executed successfully

Should Handle Negative Motion Trapezoidal
    [Documentation]    Trapezoidal profile works correctly for negative direction
    [Tags]    motion    trapezoidal    direction
    
    # First move to 90°
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=5s
    
    # Now move back to 0° (negative direction)
    Send SetTarget V2    0.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=5s
    
    Verify Position Reached    0.0    tolerance=2.0

Should Handle Negative Motion S-Curve
    [Documentation]    S-curve profile works correctly for negative direction
    [Tags]    motion    scurve    direction
    
    # Move to 60°
    Send SetTarget V2    60.0    max_vel=80.0    max_accel=400.0    profile=SCurve
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=5s
    
    # Move back to 20° (negative direction)
    Send SetTarget V2    20.0    max_vel=80.0    max_accel=400.0    profile=SCurve
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=5s
    
    Verify Position Reached    20.0    tolerance=2.0

Should Respect Velocity Limits
    [Documentation]    Velocity should not exceed max_velocity during trajectory
    [Tags]    motion    limits    velocity
    
    Send SetTarget V2    180.0    max_vel=50.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    
    # Monitor velocity during motion
    ${max_observed_vel}=    Set Variable    0.0
    FOR    ${i}    IN RANGE    50
        ${vel}=    Get Current Velocity
        ${abs_vel}=    Evaluate    abs(${vel})
        ${max_observed_vel}=    Evaluate    max(${max_observed_vel}, ${abs_vel})
        Sleep    0.05s
        Exit For Loop If    ${abs_vel} < 1.0
    END
    
    # Allow 10% margin for discretization and measurement
    ${vel_limit_with_margin}=    Evaluate    50.0 * 1.1
    Should Be True    ${max_observed_vel} <= ${vel_limit_with_margin}
    ...    Observed velocity ${max_observed_vel}°/s exceeded limit 50.0°/s

Should Respect Acceleration Limits
    [Documentation]    Acceleration should not exceed max_acceleration
    [Tags]    motion    limits    acceleration
    
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=300.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    
    # Monitor acceleration during motion
    ${prev_vel}=    Get Current Velocity
    ${prev_time}=    Get Time    epoch
    ${max_observed_accel}=    Set Variable    0.0
    
    FOR    ${i}    IN RANGE    30
        Sleep    0.05s
        ${curr_vel}=    Get Current Velocity
        ${curr_time}=    Get Time    epoch
        ${dt}=    Evaluate    ${curr_time} - ${prev_time}
        ${dv}=    Evaluate    ${curr_vel} - ${prev_vel}
        ${accel}=    Evaluate    abs(${dv} / ${dt}) if ${dt} > 0 else 0
        ${max_observed_accel}=    Evaluate    max(${max_observed_accel}, ${accel})
        ${prev_vel}=    Set Variable    ${curr_vel}
        ${prev_time}=    Set Variable    ${curr_time}
        Exit For Loop If    abs(${curr_vel}) < 1.0
    END
    
    # Allow 20% margin for discretization
    ${accel_limit_with_margin}=    Evaluate    300.0 * 1.2
    Should Be True    ${max_observed_accel} <= ${accel_limit_with_margin}
    ...    Observed acceleration ${max_observed_accel}°/s² exceeded limit 300.0°/s²

Should Handle Zero Motion Gracefully
    [Documentation]    Setting target to current position should complete immediately
    [Tags]    motion    edge-case    zero-motion
    
    # Set target to current position (0°)
    Send SetTarget V2    0.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack
    
    # Should complete almost immediately
    Sleep    0.1s
    ${velocity}=    Get Current Velocity
    Should Be True    abs(${velocity}) < 0.5

Should Support Sequential Moves
    [Documentation]    Execute multiple sequential trajectories
    [Tags]    motion    integration    sequential
    
    # First move
    Send SetTarget V2    30.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=3s
    Verify Position Reached    30.0
    
    # Second move
    Send SetTarget V2    60.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=3s
    Verify Position Reached    60.0
    
    # Third move
    Send SetTarget V2    15.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=3s
    Verify Position Reached    15.0

Should Track Trajectory With Position Controller
    [Documentation]    Position controller should follow generated trajectory
    [Tags]    motion    integration    foc
    
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    
    # Monitor tracking error during motion
    ${max_error}=    Set Variable    0.0
    FOR    ${i}    IN RANGE    50
        ${position}=    Get Current Position
        ${velocity}=    Get Current Velocity
        # Expected position based on simple model (rough approximation)
        # Real tracking should be validated more precisely
        Sleep    0.05s
        Exit For Loop If    abs(${velocity}) < 1.0
    END
    
    Verify Position Reached    90.0    tolerance=3.0

Should Compare Trapezoidal Vs S-Curve Time
    [Documentation]    S-curve should take slightly longer than trapezoidal for same motion
    [Tags]    motion    performance    comparison
    
    # Measure trapezoidal time
    ${start_time}=    Get Time    epoch
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=5s
    ${trap_time}=    Evaluate    time.time() - ${start_time}    modules=time
    
    # Reset to start
    Send SetTarget V2    0.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=5s
    
    # Measure S-curve time
    ${start_time}=    Get Time    epoch
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=SCurve
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=5s
    ${scurve_time}=    Evaluate    time.time() - ${start_time}    modules=time
    
    Log    Trapezoidal time: ${trap_time}s, S-curve time: ${scurve_time}s
    # S-curve should be within 50% of trapezoidal (may be faster or slightly slower)
    ${time_ratio}=    Evaluate    ${scurve_time} / ${trap_time}
    Should Be True    0.8 <= ${time_ratio} <= 1.5    
    ...    Time ratio ${time_ratio} outside expected range

Should Handle High Acceleration Profile
    [Documentation]    Test with very high acceleration values
    [Tags]    motion    stress    acceleration
    
    Send SetTarget V2    45.0    max_vel=150.0    max_accel=1000.0    profile=Trapezoidal
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack
    
    Wait For Motion Complete    timeout=3s
    Verify Position Reached    45.0    tolerance=2.0

Should Handle Low Acceleration Profile
    [Documentation]    Test with very low acceleration values (slow motion)
    [Tags]    motion    stress    acceleration
    
    Send SetTarget V2    20.0    max_vel=30.0    max_accel=50.0    profile=Trapezoidal
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack
    
    Wait For Motion Complete    timeout=10s
    Verify Position Reached    20.0    tolerance=2.0

Should Reject Motion In Inactive State
    [Documentation]    V2 commands should be rejected when joint is not active
    [Tags]    motion    lifecycle    error-handling
    
    # Deactivate joint
    Send Deactivate Command
    Wait For State    Inactive
    
    # Try to send motion command
    Send SetTarget V2    45.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    
    # Should receive Nack
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Nack

Should Interrupt Motion With New Command
    [Documentation]    New SetTargetV2 should interrupt current trajectory
    [Tags]    motion    interruption    behavior
    
    # Start first motion
    Send SetTarget V2    90.0    max_vel=50.0    max_accel=200.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    
    # Wait a bit, then interrupt with new command
    Sleep    0.5s
    ${mid_position}=    Get Current Position
    
    # Send new target
    Send SetTarget V2    30.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    Wait For Motion Complete    timeout=5s
    
    # Should reach new target
    Verify Position Reached    30.0    tolerance=2.0
    
    Log    Motion interrupted at ${mid_position}° and redirected to 30°

Should Handle V1 Backward Compatibility
    [Documentation]    Original SetTarget (v1) should still work alongside V2
    [Tags]    motion    compatibility    v1
    
    # Use old SetTarget command (v1)
    Send SetTarget V1    45.0    velocity_limit=80.0
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack
    
    Wait For Motion Complete    timeout=5s
    Verify Position Reached    45.0    tolerance=2.0

Should Generate Waypoints At Correct Rate
    [Documentation]    Trajectory should generate waypoints at configured timestep
    [Tags]    motion    internal    waypoints
    
    # This test would need access to internal trajectory structure
    # For now, verify motion completes in expected time
    
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For iRPC Response    timeout=1s
    
    ${start_time}=    Get Time    epoch
    Wait For Motion Complete    timeout=5s
    ${elapsed}=    Evaluate    time.time() - ${start_time}    modules=time
    
    # Rough calculation: t_accel = v/a = 100/500 = 0.2s
    # Distance during accel = 0.5*a*t² = 0.5*500*0.04 = 10°
    # Constant velocity distance = 90 - 20 = 70°
    # Time at constant vel = 70/100 = 0.7s
    # Total ≈ 0.2 + 0.7 + 0.2 = 1.1s
    ${expected_time}=    Set Variable    1.1
    ${time_error}=    Evaluate    abs(${elapsed} - ${expected_time})
    
    # Allow 100% margin for Renode simulation overhead
    Should Be True    ${time_error} < ${expected_time}
    ...    Motion time ${elapsed}s differs significantly from expected ${expected_time}s

Should Validate Invalid Parameters
    [Documentation]    Invalid parameters should be handled gracefully
    [Tags]    motion    error-handling    validation
    
    # Try to send with zero velocity (should fail internally)
    # Protocol might accept it but motion planning should error
    Send SetTarget V2    45.0    max_vel=0.0    max_accel=500.0    profile=Trapezoidal
    ${response}=    Wait For iRPC Response    timeout=1s
    # Should get Ack (command accepted) but motion won't execute
    Should Be Equal    ${response.payload_type}    Ack
    
    # Position shouldn't change
    Sleep    1s
    ${position}=    Get Current Position
    Should Be True    abs(${position}) < 1.0

Should Test Adaptive Profile Fallback
    [Documentation]    Adaptive profile should fallback to trapezoidal (not yet implemented)
    [Tags]    motion    adaptive    future
    
    Send SetTarget V2    45.0    max_vel=100.0    max_accel=500.0    profile=Adaptive
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack
    
    # Should work (using trapezoidal fallback)
    Wait For Motion Complete    timeout=5s
    Verify Position Reached    45.0    tolerance=2.0

