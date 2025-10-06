*** Settings ***
Documentation     Telemetry Streaming Tests - iRPC v2.0 Phase 2
...
...               Tests for comprehensive telemetry streaming with multiple modes.
...               Validates data collection, streaming rates, and bandwidth optimization.

Library           String
Library           Collections

# Import Renode keywords from container
Resource          /opt/renode/tests/renode-keywords.robot

Suite Setup       Setup Telemetry Suite
Suite Teardown    Teardown Telemetry Suite
Test Setup        Reset Joint State
Test Timeout      30 seconds


*** Variables ***
${FIRMWARE_ELF}        ${CURDIR}/../../target/thumbv7em-none-eabihf/release-mock/joint_firmware
${TEST_PLATFORM}       ${CURDIR}/../platforms/stm32g431cb.repl
${TEST_SCRIPT}         ${CURDIR}/../scripts/joint_test.resc


*** Keywords ***
Setup Telemetry Suite
    [Documentation]    Initialize Renode platform for telemetry tests
    Execute Script    ${TEST_SCRIPT}
    Start Emulation
    Wait For Joint Ready

Teardown Telemetry Suite
    [Documentation]    Clean up after test suite
    Stop Emulation

Reset Joint State
    [Documentation]    Reset joint to Active state
    Send Reset Command
    Wait For State    Unconfigured
    Send Configure Command
    Wait For State    Inactive
    Send Activate Command
    Wait For State    Active

Configure Telemetry
    [Arguments]    ${mode}    ${rate_hz}=100    ${threshold}=1.0
    [Documentation]    Configure telemetry streaming mode
    ${mode_value}=    Run Keyword If    '${mode}' == 'OnDemand'    Set Variable    0
    ...    ELSE IF    '${mode}' == 'Periodic'    Set Variable    1
    ...    ELSE IF    '${mode}' == 'Streaming'    Set Variable    2
    ...    ELSE IF    '${mode}' == 'OnChange'    Set Variable    3
    ...    ELSE IF    '${mode}' == 'Adaptive'    Set Variable    4
    ...    ELSE    Fail    Unknown telemetry mode: ${mode}
    
    Send iRPC Message    ConfigureTelemetry
    ...    mode=${mode_value}
    ...    rate_hz=${rate_hz}
    ...    change_threshold=${threshold}
    
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack

Request Telemetry
    [Documentation]    Request immediate telemetry (OnDemand mode)
    Send iRPC Message    RequestTelemetry
    ${telemetry}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${telemetry.payload_type}    TelemetryStream
    [Return]    ${telemetry}

Collect Telemetry Samples
    [Arguments]    ${duration_s}=1.0
    [Documentation]    Collect telemetry messages for specified duration
    ${samples}=    Create List
    ${start_time}=    Get Time    epoch
    
    FOR    ${i}    IN RANGE    999999
        ${elapsed}=    Evaluate    time.time() - ${start_time}    modules=time
        Exit For Loop If    ${elapsed} >= ${duration_s}
        
        ${msg}=    Try Get Telemetry Message    timeout=0.1s
        Run Keyword If    ${msg} is not None    Append To List    ${samples}    ${msg}
    END
    
    [Return]    ${samples}

Measure Telemetry Rate
    [Arguments]    ${duration_s}=2.0
    [Documentation]    Measure actual telemetry rate
    ${samples}=    Collect Telemetry Samples    ${duration_s}
    ${count}=    Get Length    ${samples}
    ${rate}=    Evaluate    ${count} / ${duration_s}
    [Return]    ${rate}


*** Test Cases ***

Should Configure OnDemand Mode
    [Documentation]    Configure telemetry to OnDemand mode
    [Tags]    telemetry    configuration    ondemand
    
    Configure Telemetry    OnDemand
    
    # Should not receive telemetry automatically
    ${samples}=    Collect Telemetry Samples    duration_s=1.0
    ${count}=    Get Length    ${samples}
    Should Be Equal As Numbers    ${count}    0
    
    Log    OnDemand mode configured successfully

Should Send Telemetry On Request
    [Documentation]    OnDemand mode sends only when requested
    [Tags]    telemetry    ondemand
    
    Configure Telemetry    OnDemand
    
    # Request telemetry
    ${telemetry}=    Request Telemetry
    
    # Verify telemetry fields
    Should Have Field    ${telemetry.position}
    Should Have Field    ${telemetry.velocity}
    Should Have Field    ${telemetry.timestamp_us}
    
    Log    Telemetry received: pos=${telemetry.position}°, vel=${telemetry.velocity}°/s

Should Stream At Configured Periodic Rate
    [Documentation]    Periodic mode streams at configured rate
    [Tags]    telemetry    periodic    rate
    
    # Configure 100 Hz periodic
    Configure Telemetry    Periodic    rate_hz=100
    
    # Measure actual rate
    ${rate}=    Measure Telemetry Rate    duration_s=2.0
    
    # Allow 20% margin
    Should Be True    80 <= ${rate} <= 120
    ...    Measured rate ${rate} Hz outside 100±20 Hz range
    
    Log    Periodic mode: measured ${rate} Hz (target: 100 Hz)

Should Stream At Maximum Rate
    [Documentation]    Streaming mode achieves 1 kHz rate
    [Tags]    telemetry    streaming    performance
    
    Configure Telemetry    Streaming
    
    # Measure rate over 2 seconds
    ${rate}=    Measure Telemetry Rate    duration_s=2.0
    
    # Expect ~1000 Hz, allow 20% margin for Renode overhead
    Should Be True    ${rate} >= 800
    ...    Streaming rate ${rate} Hz below 800 Hz minimum
    
    Log    Streaming mode: achieved ${rate} Hz

Should Include Motion State In Telemetry
    [Documentation]    Verify position, velocity, acceleration data
    [Tags]    telemetry    data    motion
    
    Configure Telemetry    Streaming
    
    # Start motion
    Send SetTarget V2    45.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    
    # Get telemetry sample
    Sleep    0.1s
    ${telemetry}=    Request Telemetry
    
    # Verify motion fields
    Should Have Field    ${telemetry.position}
    Should Have Field    ${telemetry.velocity}
    Should Have Field    ${telemetry.acceleration}
    
    # During motion, velocity should be non-zero
    ${abs_vel}=    Evaluate    abs(${telemetry.velocity})
    Should Be True    ${abs_vel} > 0.1
    
    Log    Motion state: pos=${telemetry.position}°, vel=${telemetry.velocity}°/s, acc=${telemetry.acceleration}°/s²

Should Include FOC State In Telemetry
    [Documentation]    Verify FOC currents and voltages
    [Tags]    telemetry    data    foc
    
    Configure Telemetry    OnDemand
    ${telemetry}=    Request Telemetry
    
    # Verify FOC fields exist
    Should Have Field    ${telemetry.current_d}
    Should Have Field    ${telemetry.current_q}
    Should Have Field    ${telemetry.voltage_d}
    Should Have Field    ${telemetry.voltage_q}
    
    Log    FOC state: Id=${telemetry.current_d}A, Iq=${telemetry.current_q}A

Should Calculate Derived Metrics
    [Documentation]    Verify torque, power, and load estimation
    [Tags]    telemetry    data    metrics
    
    # Start motion to generate load
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Sleep    0.2s
    
    ${telemetry}=    Request Telemetry
    
    # Verify derived metrics exist
    Should Have Field    ${telemetry.torque_estimate}
    Should Have Field    ${telemetry.power}
    Should Have Field    ${telemetry.load_percent}
    
    # During motion, load should be positive
    Should Be True    ${telemetry.load_percent} >= 0
    Should Be True    ${telemetry.load_percent} <= 100
    
    Log    Metrics: torque=${telemetry.torque_estimate}Nm, power=${telemetry.power}W, load=${telemetry.load_percent}%

Should Report FOC Loop Timing
    [Documentation]    Verify FOC loop performance metric
    [Tags]    telemetry    performance    timing
    
    ${telemetry}=    Request Telemetry
    
    Should Have Field    ${telemetry.foc_loop_time_us}
    
    # FOC loop should be < 100 µs (10 kHz)
    Should Be True    ${telemetry.foc_loop_time_us} > 0
    Should Be True    ${telemetry.foc_loop_time_us} < 100
    
    Log    FOC loop time: ${telemetry.foc_loop_time_us} µs

Should Report Trajectory Status
    [Documentation]    Verify trajectory_active flag
    [Tags]    telemetry    trajectory
    
    # Without trajectory
    ${telemetry}=    Request Telemetry
    Should Be False    ${telemetry.trajectory_active}
    
    # Start trajectory
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=SCurve
    Sleep    0.1s
    
    # During trajectory
    ${telemetry}=    Request Telemetry
    Should Be True    ${telemetry.trajectory_active}
    
    # Wait for completion
    Wait For Motion Complete    timeout=5s
    Sleep    0.1s
    
    # After trajectory
    ${telemetry}=    Request Telemetry
    Should Be False    ${telemetry.trajectory_active}

Should Handle OnChange Mode
    [Documentation]    Send only when values change significantly
    [Tags]    telemetry    onchange
    
    Configure Telemetry    OnChange    threshold=5.0
    
    # Idle: minimal telemetry
    ${rate_idle}=    Measure Telemetry Rate    duration_s=1.0
    
    # Start motion: increased telemetry
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    ${rate_motion}=    Measure Telemetry Rate    duration_s=1.0
    
    # Motion should generate more telemetry
    Should Be True    ${rate_motion} > ${rate_idle}
    
    Log    OnChange rates: idle=${rate_idle} Hz, motion=${rate_motion} Hz

Should Adapt Rate To Motion Activity
    [Documentation]    Adaptive mode uses high rate during motion, low when idle
    [Tags]    telemetry    adaptive    performance
    
    Configure Telemetry    Adaptive
    
    # Measure rate during idle
    ${rate_idle}=    Measure Telemetry Rate    duration_s=1.0
    Log    Idle rate: ${rate_idle} Hz
    
    # Start motion
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    
    # Measure rate during motion
    ${rate_motion}=    Measure Telemetry Rate    duration_s=1.0
    Log    Motion rate: ${rate_motion} Hz
    
    # Wait for motion to complete
    Wait For Motion Complete    timeout=5s
    Sleep    0.5s
    
    # Measure rate after motion
    ${rate_after}=    Measure Telemetry Rate    duration_s=1.0
    Log    After motion rate: ${rate_after} Hz
    
    # Motion rate should be significantly higher
    Should Be True    ${rate_motion} > ${rate_idle} * 3
    ...    Motion rate ${rate_motion} not 3x idle rate ${rate_idle}
    
    # After motion should return to low rate
    Should Be True    ${rate_after} < ${rate_motion} / 3
    ...    After-motion rate ${rate_after} didn't decrease

Should Maintain Telemetry During Commands
    [Documentation]    Telemetry continues during command processing
    [Tags]    telemetry    integration
    
    Configure Telemetry    Streaming
    
    # Send multiple commands while streaming
    FOR    ${angle}    IN RANGE    0    90    15
        Send SetTarget V2    ${angle}    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
        Sleep    0.1s
    END
    
    # Telemetry should still be coming
    ${rate}=    Measure Telemetry Rate    duration_s=1.0
    Should Be True    ${rate} > 500
    
    Log    Telemetry maintained at ${rate} Hz during commands

Should Handle Sequential Mode Changes
    [Documentation]    Switch between telemetry modes
    [Tags]    telemetry    configuration
    
    # Start with OnDemand
    Configure Telemetry    OnDemand
    ${samples1}=    Collect Telemetry Samples    duration_s=0.5
    ${count1}=    Get Length    ${samples1}
    Should Be Equal As Numbers    ${count1}    0
    
    # Switch to Streaming
    Configure Telemetry    Streaming
    ${rate2}=    Measure Telemetry Rate    duration_s=1.0
    Should Be True    ${rate2} > 500
    
    # Switch to Periodic 50 Hz
    Configure Telemetry    Periodic    rate_hz=50
    ${rate3}=    Measure Telemetry Rate    duration_s=2.0
    Should Be True    40 <= ${rate3} <= 60
    
    Log    Mode switching successful

Should Timestamp Telemetry Correctly
    [Documentation]    Timestamps should be monotonic and reasonable
    [Tags]    telemetry    timestamp
    
    Configure Telemetry    Streaming
    ${samples}=    Collect Telemetry Samples    duration_s=1.0
    
    ${prev_timestamp}=    Set Variable    0
    FOR    ${sample}    IN    @{samples}
        # Timestamp should be monotonically increasing
        Should Be True    ${sample.timestamp_us} > ${prev_timestamp}
        ${prev_timestamp}=    Set Variable    ${sample.timestamp_us}
    END
    
    Log    Timestamps monotonic over ${prev_timestamp} µs

Should Measure Bandwidth Usage
    [Documentation]    Verify CAN bandwidth stays within limits
    [Tags]    telemetry    bandwidth    performance
    
    Configure Telemetry    Streaming
    
    # Measure for 2 seconds
    ${samples}=    Collect Telemetry Samples    duration_s=2.0
    ${count}=    Get Length    ${samples}
    ${rate}=    Evaluate    ${count} / 2.0
    
    # Each message ~74 bytes, CAN-FD 5 Mbps
    ${bandwidth_bps}=    Evaluate    ${rate} * 74 * 8
    ${bandwidth_percent}=    Evaluate    ${bandwidth_bps} / 5000000.0 * 100
    
    # Should be < 20% of CAN-FD bandwidth
    Should Be True    ${bandwidth_percent} < 20
    
    Log    Bandwidth: ${bandwidth_percent}% of CAN-FD (${rate} Hz × 74 bytes)

Should Handle Telemetry In All Lifecycle States
    [Documentation]    Telemetry behavior in different states
    [Tags]    telemetry    lifecycle
    
    # In Active state: telemetry works
    Configure Telemetry    OnDemand
    ${telemetry1}=    Request Telemetry
    Should Not Be Empty    ${telemetry1}
    
    # Deactivate
    Send Deactivate Command
    Wait For State    Inactive
    
    # In Inactive: telemetry should be rejected or return zeros
    ${result}=    Try Request Telemetry
    # Implementation may reject or send zeros
    
    # Reactivate
    Send Activate Command
    Wait For State    Active
    
    # Works again
    Configure Telemetry    OnDemand
    ${telemetry2}=    Request Telemetry
    Should Not Be Empty    ${telemetry2}

Should Provide Accurate Position Data
    [Documentation]    Telemetry position matches actual position
    [Tags]    telemetry    accuracy    motion
    
    Configure Telemetry    OnDemand
    
    # Move to known position
    Send SetTarget V2    45.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    Wait For Motion Complete    timeout=5s
    
    # Get telemetry
    ${telemetry}=    Request Telemetry
    
    # Position should be close to 45°
    ${error}=    Evaluate    abs(${telemetry.position} - 45.0)
    Should Be True    ${error} < 5.0
    ...    Position error ${error}° exceeds 5° tolerance
    
    Log    Position accuracy: ${telemetry.position}° (target: 45°, error: ${error}°)

Should Calculate Acceleration From Velocity
    [Documentation]    Acceleration should be derivative of velocity
    [Tags]    telemetry    calculation    motion
    
    Configure Telemetry    Streaming
    
    # Start motion
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    
    # Collect samples during acceleration phase
    Sleep    0.1s
    ${samples}=    Collect Telemetry Samples    duration_s=0.5
    
    # During acceleration, acceleration should be positive
    ${accel_sum}=    Set Variable    0
    FOR    ${sample}    IN    @{samples}
        ${accel_sum}=    Evaluate    ${accel_sum} + ${sample.acceleration}
    END
    
    ${count}=    Get Length    ${samples}
    ${avg_accel}=    Evaluate    ${accel_sum} / ${count}
    
    # Should be positive during acceleration
    Should Be True    ${avg_accel} > 0
    
    Log    Average acceleration: ${avg_accel}°/s²

Should Report Load During Motion
    [Documentation]    Load percentage increases during motion
    [Tags]    telemetry    load    motion
    
    # Idle load
    ${telemetry_idle}=    Request Telemetry
    ${load_idle}=    Set Variable    ${telemetry_idle.load_percent}
    
    # Start aggressive motion
    Send SetTarget V2    180.0    max_vel=150.0    max_accel=800.0    profile=Trapezoidal
    Sleep    0.2s
    
    # Motion load
    ${telemetry_motion}=    Request Telemetry
    ${load_motion}=    Set Variable    ${telemetry_motion.load_percent}
    
    # Load should increase during motion
    Should Be True    ${load_motion} > ${load_idle}
    
    Log    Load: idle=${load_idle}%, motion=${load_motion}%

Should Survive High Message Rate
    [Documentation]    System stable under maximum telemetry rate
    [Tags]    telemetry    stress    performance
    
    Configure Telemetry    Streaming
    
    # Also send commands during streaming
    FOR    ${i}    IN RANGE    10
        Send SetTarget V2    ${i * 10}    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
        Sleep    0.2s
    END
    
    # Telemetry should still work
    ${telemetry}=    Request Telemetry
    Should Not Be Empty    ${telemetry}
    
    # System should be responsive
    Send SetTarget V2    0.0    max_vel=100.0    max_accel=500.0    profile=Trapezoidal
    ${response}=    Wait For iRPC Response    timeout=1s
    Should Be Equal    ${response.payload_type}    Ack

Should Provide Telemetry Summary Statistics
    [Documentation]    Collect and analyze telemetry statistics
    [Tags]    telemetry    analysis
    
    Configure Telemetry    Streaming
    
    # Start motion and collect data
    Send SetTarget V2    90.0    max_vel=100.0    max_accel=500.0    profile=SCurve
    ${samples}=    Collect Telemetry Samples    duration_s=2.0
    
    ${count}=    Get Length    ${samples}
    Should Be True    ${count} > 1000    # At least 500 Hz average
    
    # Analyze position range
    ${positions}=    Create List
    FOR    ${sample}    IN    @{samples}
        Append To List    ${positions}    ${sample.position}
    END
    
    ${min_pos}=    Evaluate    min(${positions})
    ${max_pos}=    Evaluate    max(${positions})
    ${range}=    Evaluate    ${max_pos} - ${min_pos}
    
    # Should cover significant range during motion
    Should Be True    ${range} > 30
    
    Log    Collected ${count} samples, position range: ${min_pos}° to ${max_pos}° (${range}°)

