*** Settings ***
Documentation    Example motion planning test with FOC visualization
Resource         test_visualization_keywords.robot
Resource         renode_helpers.robot
Suite Setup      Setup Test Suite
Suite Teardown   Teardown Test Suite
Test Setup       Setup Single Test
Test Teardown    Teardown Single Test

*** Variables ***
${FIRMWARE_ELF}    ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware

*** Test Cases ***
Test Trapezoidal Motion Profile With Visualization
    [Documentation]    Test trapezoidal profile with full FOC data collection
    [Tags]    motion    visualization    foc
    
    # Start data collection
    Start Test Data Collection    trapezoidal_profile_foc
    
    # Initialize joint
    Send iRPC Command    Enable    node_id=0x10
    Sleep    10ms
    
    # Configure motion planner
    ${target_pos} =    Set Variable    ${1.57}    # 90 degrees
    ${max_vel} =    Set Variable    ${2.0}    # 2 rad/s
    ${max_accel} =    Set Variable    ${5.0}    # 5 rad/s²
    
    Log    Sending SetTargetV2: pos=${target_pos}, vel=${max_vel}, accel=${max_accel}    console=yes
    
    Send SetTargetV2 Command    ${target_pos}    ${max_vel}    ${max_accel}    profile=Trapezoidal
    
    # Record FOC data during motion (500ms at 10kHz = 5000 samples)
    Record Multiple FOC Snapshots    duration_ms=500    sample_rate_hz=10000
    
    # Verify arrival
    ${final_pos} =    Get Encoder Position
    Should Be Close    ${final_pos}    ${target_pos}    tolerance=0.01
    
    # Stop data collection
    Stop Test Data Collection
    
    # Generate report
    Generate Test Report    trapezoidal_profile_foc
    
    Log    ✓ Test complete with FOC visualization    console=yes

Test S-Curve Motion Profile With Visualization
    [Documentation]    Test S-curve profile with full FOC data collection
    [Tags]    motion    visualization    foc
    
    # Start data collection
    Start Test Data Collection    scurve_profile_foc
    
    # Initialize joint
    Send iRPC Command    Enable    node_id=0x10
    Sleep    10ms
    
    # Configure S-curve motion
    ${target_pos} =    Set Variable    ${3.14}    # 180 degrees
    ${max_vel} =    Set Variable    ${3.0}
    ${max_accel} =    Set Variable    ${10.0}
    ${max_jerk} =    Set Variable    ${50.0}
    
    Log    Sending SetTargetV2 (S-curve): pos=${target_pos}    console=yes
    
    Send SetTargetV2 Command    ${target_pos}    ${max_vel}    ${max_accel}
    ...                         jerk=${max_jerk}    profile=SCurve
    
    # Record FOC data during motion
    Record Multiple FOC Snapshots    duration_ms=800    sample_rate_hz=10000
    
    # Verify arrival
    ${final_pos} =    Get Encoder Position
    Should Be Close    ${final_pos}    ${target_pos}    tolerance=0.01
    
    # Stop data collection
    Stop Test Data Collection
    
    # Generate report
    Generate Test Report    scurve_profile_foc
    
    Log    ✓ S-curve test complete with FOC visualization    console=yes

Test Adaptive Control With Visualization
    [Documentation]    Test coolStep/dcStep with load changes and FOC visualization
    [Tags]    adaptive    visualization    foc
    
    # Start data collection
    Start Test Data Collection    adaptive_control_foc
    
    # Initialize joint
    Send iRPC Command    Enable    node_id=0x10
    Sleep    10ms
    
    # Enable adaptive control
    Send ConfigureAdaptive Command    
    ...    enable_coolstep=${True}
    ...    enable_dcstep=${True}
    ...    enable_stallguard=${True}
    
    # Move to position with adaptive control
    ${target_pos} =    Set Variable    ${1.57}
    Send SetTargetV2 Command    ${target_pos}    ${2.0}    ${5.0}    profile=Trapezoidal
    
    # Record during motion
    Record Multiple FOC Snapshots    duration_ms=300    sample_rate_hz=10000
    
    # Apply load disturbance (via motor simulator)
    Log    Applying external load disturbance    console=yes
    Set Motor External Load    ${0.2}    # 0.2 Nm
    
    # Record response to load
    Record Multiple FOC Snapshots    duration_ms=200    sample_rate_hz=10000
    
    # Remove load
    Set Motor External Load    ${0.0}
    
    # Record recovery
    Record Multiple FOC Snapshots    duration_ms=200    sample_rate_hz=10000
    
    # Stop data collection
    Stop Test Data Collection
    
    # Generate report (will show coolStep current reduction, load estimation)
    Generate Test Report    adaptive_control_foc
    
    Log    ✓ Adaptive control test complete with FOC visualization    console=yes

Test High-Speed Motion With Visualization
    [Documentation]    Test high-speed motion to verify FOC performance under stress
    [Tags]    motion    performance    visualization
    
    # Start data collection
    Start Test Data Collection    high_speed_motion_foc
    
    # Initialize joint
    Send iRPC Command    Enable    node_id=0x10
    Sleep    10ms
    
    # High-speed motion
    ${target_pos} =    Set Variable    ${6.28}    # 360 degrees
    ${max_vel} =    Set Variable    ${10.0}    # 10 rad/s (very fast)
    ${max_accel} =    Set Variable    ${50.0}    # 50 rad/s²
    
    Log    High-speed test: ${max_vel} rad/s    console=yes
    
    Send SetTargetV2 Command    ${target_pos}    ${max_vel}    ${max_accel}    profile=SCurve
    
    # Record FOC data
    Record Multiple FOC Snapshots    duration_ms=1000    sample_rate_hz=10000
    
    # Stop data collection
    Stop Test Data Collection
    
    # Generate report (will show current spikes, tracking error)
    Generate Test Report    high_speed_motion_foc
    
    Log    ✓ High-speed motion test complete    console=yes

*** Keywords ***
Setup Test Suite
    [Documentation]    Initialize Renode and load firmware
    Log    Setting up test suite...    console=yes
    
    # Load Renode platform
    Execute Command    include @${CURDIR}/../scripts/joint_test.resc
    
    # Load firmware
    Execute Command    sysbus LoadELF @${FIRMWARE_ELF}
    
    # Start emulation
    Execute Command    start
    
    Log    Renode emulation started    console=yes

Teardown Test Suite
    [Documentation]    Cleanup and generate summary report
    Log    Tearing down test suite...    console=yes
    
    # Stop emulation
    Execute Command    quit
    
    # Generate summary report for all tests
    Generate Suite Summary Report
    
    Log    Test suite complete - reports generated    console=yes

Setup Single Test
    [Documentation]    Reset emulation before each test
    Execute Command    machine Reset

Teardown Single Test
    [Documentation]    Cleanup after each test
    # Stop any active data collection
    Run Keyword And Ignore Error    Stop Test Data Collection

# Mock peripheral control keywords
Send SetTargetV2 Command
    [Documentation]    Send SetTargetV2 iRPC command
    [Arguments]    ${position}    ${velocity}    ${acceleration}    ${jerk}=0.0    ${profile}=Trapezoidal
    
    # TODO: Implement actual iRPC message sending via CAN test device
    Log    SetTargetV2: pos=${position}, vel=${velocity}, accel=${acceleration}, jerk=${jerk}, profile=${profile}    console=yes

Send ConfigureAdaptive Command
    [Documentation]    Send ConfigureAdaptive iRPC command
    [Arguments]    ${enable_coolstep}=${True}    ${enable_dcstep}=${True}    ${enable_stallguard}=${True}
    
    # TODO: Implement actual iRPC message sending
    Log    ConfigureAdaptive: coolStep=${enable_coolstep}, dcStep=${enable_dcstep}, stallGuard=${enable_stallguard}    console=yes

Set Motor External Load
    [Documentation]    Apply external load to motor simulator
    [Arguments]    ${load_nm}
    
    # TODO: Call motor simulator API
    Log    Setting external load: ${load_nm} Nm    console=yes

Should Be Close
    [Documentation]    Assert that two float values are within tolerance
    [Arguments]    ${actual}    ${expected}    ${tolerance}=0.01
    
    ${diff} =    Evaluate    abs(${actual} - ${expected})
    Should Be True    ${diff} <= ${tolerance}    msg=Expected ${expected}, got ${actual} (diff: ${diff})

