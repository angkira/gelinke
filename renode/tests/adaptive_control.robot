*** Settings ***
Documentation     Adaptive Control Tests - iRPC v2.0 Phase 3
...
...               Tests for coolStep, dcStep, and stallGuard adaptive control features.
...               Validates load estimation, power savings, stall detection, and predictive diagnostics.

Resource          test_helpers.robot
Library           String
Library           Collections

Suite Setup       Setup Adaptive Control Suite
Suite Teardown    Teardown Adaptive Control Suite
Test Setup        Reset Joint And Enable Adaptive
Test Timeout      30 seconds


*** Variables ***
${FIRMWARE_ELF}        ${CURDIR}/../../target/thumbv7em-none-eabihf/release-mock/joint_firmware
${TEST_PLATFORM}       ${CURDIR}/../platforms/stm32g431cb.repl
${TEST_SCRIPT}         ${CURDIR}/../scripts/joint_test.resc

# Adaptive control parameters
${DEFAULT_COOLSTEP_THRESHOLD}      30.0     # % load
${DEFAULT_DCSTEP_THRESHOLD}        70.0     # % load
${DEFAULT_STALLGUARD_CURRENT}      2.5      # Amperes
${DEFAULT_STALLGUARD_VELOCITY}     3.0      # deg/s

# Load simulation parameters
${LOW_LOAD}            10.0     # %
${MEDIUM_LOAD}         50.0     # %
${HIGH_LOAD}           80.0     # %
${CRITICAL_LOAD}       95.0     # %

# Tolerances
${LOAD_TOLERANCE}      5.0      # %
${CURRENT_TOLERANCE}   0.1      # A
${POWER_TOLERANCE}     5.0      # %


*** Keywords ***
Setup Adaptive Control Suite
    [Documentation]    Initialize Renode platform for adaptive control tests
    Execute Script    ${TEST_SCRIPT}
    Start Emulation
    Wait For Joint Ready
    Log    Adaptive control test suite initialized

Teardown Adaptive Control Suite
    [Documentation]    Clean up after test suite
    Stop Emulation
    Log    Adaptive control test suite completed

Reset Joint And Enable Adaptive
    [Documentation]    Reset joint and enable all adaptive features
    Send Reset Command
    Wait For State    Unconfigured
    Send Configure Command
    Wait For State    Inactive
    Send Activate Command
    Wait For State    Active
    
    # Enable all adaptive features with default settings
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    coolstep_min_current=0.3
    ...    coolstep_threshold=${DEFAULT_COOLSTEP_THRESHOLD}
    ...    dcstep_enable=true
    ...    dcstep_threshold=${DEFAULT_DCSTEP_THRESHOLD}
    ...    dcstep_max_derating=0.2
    ...    stallguard_enable=true
    ...    stallguard_current=${DEFAULT_STALLGUARD_CURRENT}
    ...    stallguard_velocity=${DEFAULT_STALLGUARD_VELOCITY}

Configure Adaptive Features
    [Arguments]    ${coolstep_enable}=true    ${coolstep_min_current}=0.3    ${coolstep_threshold}=30.0
    ...            ${dcstep_enable}=true    ${dcstep_threshold}=70.0    ${dcstep_max_derating}=0.2
    ...            ${stallguard_enable}=true    ${stallguard_current}=2.5    ${stallguard_velocity}=3.0
    [Documentation]    Send ConfigureAdaptive message
    
    Send iRPC Message    ConfigureAdaptive
    ...    coolstep_enable=${coolstep_enable}
    ...    coolstep_min_current=${coolstep_min_current}
    ...    coolstep_threshold=${coolstep_threshold}
    ...    dcstep_enable=${dcstep_enable}
    ...    dcstep_threshold=${dcstep_threshold}
    ...    dcstep_max_derating=${dcstep_max_derating}
    ...    stallguard_enable=${stallguard_enable}
    ...    stallguard_current_threshold=${stallguard_current}
    ...    stallguard_velocity_threshold=${stallguard_velocity}
    
    Sleep    0.1s    # Allow configuration to take effect

Request Adaptive Status
    [Documentation]    Request and return adaptive status
    Send iRPC Message    RequestAdaptiveStatus
    ${status}=    Receive iRPC Message    AdaptiveStatus    timeout=1s
    RETURN    ${status}

Simulate Load
    [Arguments]    ${load_percent}
    [Documentation]    Simulate external load on joint
    # This would interact with Renode mock to adjust simulated load
    Set Load Percentage    ${load_percent}
    Sleep    0.1s    # Allow load to stabilize

Verify Load Estimation
    [Arguments]    ${expected_load}    ${tolerance}=${LOAD_TOLERANCE}
    [Documentation]    Verify load estimation accuracy
    ${status}=    Request Adaptive Status
    ${actual_load}=    Get From Dictionary    ${status}    load_percent
    ${error}=    Evaluate    abs(${actual_load} - ${expected_load})
    Should Be True    ${error} < ${tolerance}
    ...    Load estimation error ${error}% exceeds tolerance ${tolerance}%

Verify Power Savings
    [Arguments]    ${min_savings}
    [Documentation]    Verify minimum power savings percentage
    ${status}=    Request Adaptive Status
    ${savings}=    Get From Dictionary    ${status}    power_savings_percent
    Should Be True    ${savings} >= ${min_savings}
    ...    Power savings ${savings}% below minimum ${min_savings}%


*** Test Cases ***

# ============================================================================
# Configuration Tests
# ============================================================================

Test Configure All Features Enabled
    [Documentation]    Verify ConfigureAdaptive enables all features
    [Tags]    adaptive    configuration    basic
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    dcstep_enable=true
    ...    stallguard_enable=true
    
    ${status}=    Request Adaptive Status
    ${coolstep_enabled}=    Get From Dictionary    ${status}    coolstep_enabled
    ${dcstep_enabled}=    Get From Dictionary    ${status}    dcstep_enabled
    ${stallguard_enabled}=    Get From Dictionary    ${status}    stallguard_enabled
    
    Should Be True    ${coolstep_enabled}
    Should Be True    ${dcstep_enabled}
    Should Be True    ${stallguard_enabled}

Test Configure Selective Features
    [Documentation]    Verify individual feature enable/disable
    [Tags]    adaptive    configuration
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    dcstep_enable=false
    ...    stallguard_enable=true
    
    ${status}=    Request Adaptive Status
    ${coolstep_enabled}=    Get From Dictionary    ${status}    coolstep_enabled
    ${dcstep_enabled}=    Get From Dictionary    ${status}    dcstep_enabled
    ${stallguard_enabled}=    Get From Dictionary    ${status}    stallguard_enabled
    
    Should Be True    ${coolstep_enabled}
    Should Not Be True    ${dcstep_enabled}
    Should Be True    ${stallguard_enabled}

Test Configure Thresholds
    [Documentation]    Verify custom threshold configuration
    [Tags]    adaptive    configuration
    
    Configure Adaptive Features
    ...    coolstep_threshold=40.0
    ...    dcstep_threshold=80.0
    
    # Verify by testing behavior at different loads
    Simulate Load    ${MEDIUM_LOAD}
    ${status}=    Request Adaptive Status
    ${current_scale}=    Get From Dictionary    ${status}    current_scale
    
    # With 40% threshold, 50% load should see some reduction
    Should Be True    ${current_scale} < 1.0

# ============================================================================
# coolStep Tests
# ============================================================================

Test CoolStep Low Load Reduction
    [Documentation]    Verify coolStep reduces current at low load
    [Tags]    adaptive    coolstep    power
    
    Configure Adaptive Features    coolstep_enable=true
    
    # Simulate very low load
    Simulate Load    ${LOW_LOAD}
    Sleep    0.5s    # Allow coolStep to adapt
    
    ${status}=    Request Adaptive Status
    ${current_scale}=    Get From Dictionary    ${status}    current_scale
    
    # Should significantly reduce current at 10% load
    Should Be True    ${current_scale} < 0.6
    ...    Current scale ${current_scale} not reduced enough at low load

Test CoolStep High Load Full Current
    [Documentation]    Verify coolStep maintains full current at high load
    [Tags]    adaptive    coolstep    power
    
    Configure Adaptive Features    coolstep_enable=true
    
    # Simulate high load
    Simulate Load    ${HIGH_LOAD}
    Sleep    0.5s
    
    ${status}=    Request Adaptive Status
    ${current_scale}=    Get From Dictionary    ${status}    current_scale
    
    # Should use full or near-full current at high load
    Should Be True    ${current_scale} > 0.9
    ...    Current scale ${current_scale} too low at high load

Test CoolStep Power Savings Calculation
    [Documentation]    Verify power savings percentage is calculated
    [Tags]    adaptive    coolstep    power
    
    Configure Adaptive Features    coolstep_enable=true
    
    Simulate Load    ${LOW_LOAD}
    Sleep    0.5s
    
    ${status}=    Request Adaptive Status
    ${savings}=    Get From Dictionary    ${status}    power_savings_percent
    
    # Should show significant savings at low load
    Should Be True    ${savings} > 30.0
    ...    Power savings ${savings}% too low at low load

Test CoolStep Energy Accumulation
    [Documentation]    Verify energy saved accumulates over time
    [Tags]    adaptive    coolstep    power
    
    Configure Adaptive Features    coolstep_enable=true
    
    Simulate Load    ${LOW_LOAD}
    
    # Wait and accumulate energy
    Sleep    2s
    
    ${status}=    Request Adaptive Status
    ${energy_saved}=    Get From Dictionary    ${status}    energy_saved_wh
    
    # Should accumulate some energy over 2 seconds
    Should Be True    ${energy_saved} > 0.0
    ...    No energy savings accumulated

Test CoolStep Minimum Current Limit
    [Documentation]    Verify coolStep respects minimum current limit
    [Tags]    adaptive    coolstep    safety
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    coolstep_min_current=0.4
    
    Simulate Load    5.0    # Very low load
    Sleep    0.5s
    
    ${status}=    Request Adaptive Status
    ${current_scale}=    Get From Dictionary    ${status}    current_scale
    
    # Should not go below 40% minimum
    Should Be True    ${current_scale} >= 0.4
    ...    Current scale ${current_scale} below minimum 0.4

Test CoolStep Threshold Behavior
    [Documentation]    Verify current reduction starts at threshold
    [Tags]    adaptive    coolstep    threshold
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    coolstep_threshold=30.0
    
    # Test just below threshold
    Simulate Load    25.0
    Sleep    0.5s
    ${status1}=    Request Adaptive Status
    ${scale1}=    Get From Dictionary    ${status1}    current_scale
    
    # Test just above threshold
    Simulate Load    35.0
    Sleep    0.5s
    ${status2}=    Request Adaptive Status
    ${scale2}=    Get From Dictionary    ${status2}    current_scale
    
    # Below threshold should have more reduction
    Should Be True    ${scale1} < ${scale2}

# ============================================================================
# dcStep Tests
# ============================================================================

Test DcStep Normal Load No Derating
    [Documentation]    Verify dcStep does not derate at normal load
    [Tags]    adaptive    dcstep    velocity
    
    Configure Adaptive Features    dcstep_enable=true
    
    # Simulate normal load below threshold
    Simulate Load    ${MEDIUM_LOAD}
    Sleep    0.2s
    
    ${status}=    Request Adaptive Status
    ${velocity_scale}=    Get From Dictionary    ${status}    velocity_scale
    ${derating}=    Get From Dictionary    ${status}    dcstep_derating
    
    # Should be at full velocity
    Should Be Equal    ${velocity_scale}    1.0
    Should Not Be True    ${derating}

Test DcStep High Load Derating
    [Documentation]    Verify dcStep derates velocity at high load
    [Tags]    adaptive    dcstep    velocity
    
    Configure Adaptive Features    dcstep_enable=true
    
    # Simulate high load above threshold (70%)
    Simulate Load    ${HIGH_LOAD}
    Sleep    0.2s
    
    ${status}=    Request Adaptive Status
    ${velocity_scale}=    Get From Dictionary    ${status}    velocity_scale
    ${derating}=    Get From Dictionary    ${status}    dcstep_derating
    
    # Should be derating
    Should Be True    ${velocity_scale} < 1.0
    Should Be True    ${derating}

Test DcStep Critical Load Minimum Velocity
    [Documentation]    Verify dcStep limits to minimum velocity at critical load
    [Tags]    adaptive    dcstep    velocity    safety
    
    Configure Adaptive Features
    ...    dcstep_enable=true
    ...    dcstep_max_derating=0.2
    
    # Simulate critical load
    Simulate Load    ${CRITICAL_LOAD}
    Sleep    0.2s
    
    ${status}=    Request Adaptive Status
    ${velocity_scale}=    Get From Dictionary    ${status}    velocity_scale
    
    # Should be at minimum (1.0 - 0.2 = 0.8)
    Should Be True    ${velocity_scale} >= 0.8
    Should Be True    ${velocity_scale} <= 0.85
    ...    Velocity scale ${velocity_scale} outside expected range

Test DcStep Derating Recovery
    [Documentation]    Verify dcStep recovers when load decreases
    [Tags]    adaptive    dcstep    velocity
    
    Configure Adaptive Features    dcstep_enable=true
    
    # Start with high load
    Simulate Load    ${HIGH_LOAD}
    Sleep    0.2s
    ${status1}=    Request Adaptive Status
    ${scale1}=    Get From Dictionary    ${status1}    velocity_scale
    
    # Reduce load
    Simulate Load    ${MEDIUM_LOAD}
    Sleep    0.2s
    ${status2}=    Request Adaptive Status
    ${scale2}=    Get From Dictionary    ${status2}    velocity_scale
    
    # Should recover to higher velocity
    Should Be True    ${scale2} > ${scale1}

Test DcStep Linear Derating Profile
    [Documentation]    Verify dcStep derates linearly between thresholds
    [Tags]    adaptive    dcstep    velocity
    
    Configure Adaptive Features
    ...    dcstep_enable=true
    ...    dcstep_threshold=70.0
    
    # Test at 80% load (halfway between 70% and 90%)
    Simulate Load    80.0
    Sleep    0.2s
    ${status}=    Request Adaptive Status
    ${velocity_scale}=    Get From Dictionary    ${status}    velocity_scale
    
    # Should be approximately halfway reduced
    Should Be True    ${velocity_scale} > 0.85
    Should Be True    ${velocity_scale} < 0.95

# ============================================================================
# stallGuard Tests
# ============================================================================

Test StallGuard Normal Operation
    [Documentation]    Verify stallGuard shows normal status during normal operation
    [Tags]    adaptive    stallguard    stall
    
    Configure Adaptive Features    stallguard_enable=true
    
    # Normal operation: moderate current, normal velocity
    Simulate Normal Motion    velocity=30.0    current=1.5
    Sleep    0.2s
    
    ${status}=    Request Adaptive Status
    ${stall_status}=    Get From Dictionary    ${status}    stall_status
    
    Should Be Equal    ${stall_status}    Normal

Test StallGuard Warning Detection
    [Documentation]    Verify stallGuard detects warning condition
    [Tags]    adaptive    stallguard    stall
    
    Configure Adaptive Features    stallguard_enable=true
    
    # High load condition: high current, low velocity
    Simulate Load    85.0
    Simulate Motion    velocity=5.0    current=2.4
    Sleep    0.15s
    
    ${status}=    Request Adaptive Status
    ${stall_status}=    Get From Dictionary    ${status}    stall_status
    
    # Should be warning or stalled
    Should Not Be Equal    ${stall_status}    Normal

Test StallGuard Stall Detection
    [Documentation]    Verify stallGuard detects full stall
    [Tags]    adaptive    stallguard    stall
    
    Configure Adaptive Features    stallguard_enable=true
    
    # Stall condition: very high current, nearly zero velocity
    Simulate Blocked Motor    current=3.0    velocity=0.5
    Sleep    0.15s
    
    ${status}=    Request Adaptive Status
    ${stall_status}=    Get From Dictionary    ${status}    stall_status
    ${confidence}=    Get From Dictionary    ${status}    stall_confidence
    
    Should Be Equal    ${stall_status}    Stalled
    Should Be True    ${confidence} > 50.0

Test StallGuard Confidence Metric
    [Documentation]    Verify stallGuard confidence increases over time
    [Tags]    adaptive    stallguard    stall
    
    Configure Adaptive Features    stallguard_enable=true
    
    # Start stall condition
    Simulate Blocked Motor    current=2.8    velocity=1.0
    
    # Check confidence increasing
    Sleep    0.05s
    ${status1}=    Request Adaptive Status
    ${conf1}=    Get From Dictionary    ${status1}    stall_confidence
    
    Sleep    0.1s
    ${status2}=    Request Adaptive Status
    ${conf2}=    Get From Dictionary    ${status2}    stall_confidence
    
    # Confidence should increase
    Should Be True    ${conf2} > ${conf1}

Test StallGuard Threshold Configuration
    [Documentation]    Verify stallGuard respects custom thresholds
    [Tags]    adaptive    stallguard    configuration
    
    Configure Adaptive Features
    ...    stallguard_enable=true
    ...    stallguard_current=3.0
    ...    stallguard_velocity=5.0
    
    # Condition below thresholds: should be normal
    Simulate Motion    current=2.5    velocity=6.0
    Sleep    0.1s
    ${status}=    Request Adaptive Status
    ${stall_status}=    Get From Dictionary    ${status}    stall_status
    
    Should Be Equal    ${stall_status}    Normal

# ============================================================================
# Integration Tests
# ============================================================================

Test Combined Coolstep And Dcstep
    [Documentation]    Verify coolStep and dcStep work together
    [Tags]    adaptive    integration    coolstep    dcstep
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    dcstep_enable=true
    
    # Simulate low load: coolStep should reduce current, dcStep inactive
    Simulate Load    ${LOW_LOAD}
    Sleep    0.5s
    ${status1}=    Request Adaptive Status
    ${current_scale1}=    Get From Dictionary    ${status1}    current_scale
    ${velocity_scale1}=    Get From Dictionary    ${status1}    velocity_scale
    
    Should Be True    ${current_scale1} < 0.7    # coolStep active
    Should Be Equal    ${velocity_scale1}    1.0    # dcStep inactive
    
    # Simulate high load: coolStep full current, dcStep derating
    Simulate Load    ${HIGH_LOAD}
    Sleep    0.5s
    ${status2}=    Request Adaptive Status
    ${current_scale2}=    Get From Dictionary    ${status2}    current_scale
    ${velocity_scale2}=    Get From Dictionary    ${status2}    velocity_scale
    
    Should Be True    ${current_scale2} > 0.9    # coolStep inactive
    Should Be True    ${velocity_scale2} < 1.0   # dcStep active

Test All Features Under Varying Load
    [Documentation]    Test all adaptive features with load transitions
    [Tags]    adaptive    integration    comprehensive
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    dcstep_enable=true
    ...    stallguard_enable=true
    
    # Low load phase
    Simulate Load    ${LOW_LOAD}
    Sleep    0.5s
    ${status_low}=    Request Adaptive Status
    
    # Medium load phase
    Simulate Load    ${MEDIUM_LOAD}
    Sleep    0.5s
    ${status_med}=    Request Adaptive Status
    
    # High load phase
    Simulate Load    ${HIGH_LOAD}
    Sleep    0.5s
    ${status_high}=    Request Adaptive Status
    
    # Verify progressive adaptation
    ${scale_low}=    Get From Dictionary    ${status_low}    current_scale
    ${scale_med}=    Get From Dictionary    ${status_med}    current_scale
    ${scale_high}=    Get From Dictionary    ${status_high}    current_scale
    
    # Current scale should increase with load
    Should Be True    ${scale_low} < ${scale_med} < ${scale_high}

Test Adaptive Control Performance
    [Documentation]    Verify adaptive control meets performance targets
    [Tags]    adaptive    performance    timing
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    dcstep_enable=true
    ...    stallguard_enable=true
    
    # Measure update timing
    ${timing}=    Measure FOC Loop Time
    
    # Should be under 50 µs overhead target
    Should Be True    ${timing} < 50.0
    ...    FOC loop time ${timing} µs exceeds 50 µs target

Test Adaptive Status Message Format
    [Documentation]    Verify AdaptiveStatus message contains all fields
    [Tags]    adaptive    protocol    telemetry
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    dcstep_enable=true
    ...    stallguard_enable=true
    
    ${status}=    Request Adaptive Status
    
    # Verify all required fields present
    Dictionary Should Contain Key    ${status}    load_percent
    Dictionary Should Contain Key    ${status}    current_scale
    Dictionary Should Contain Key    ${status}    coolstep_enabled
    Dictionary Should Contain Key    ${status}    power_savings_percent
    Dictionary Should Contain Key    ${status}    energy_saved_wh
    Dictionary Should Contain Key    ${status}    velocity_scale
    Dictionary Should Contain Key    ${status}    dcstep_enabled
    Dictionary Should Contain Key    ${status}    dcstep_derating
    Dictionary Should Contain Key    ${status}    stall_status
    Dictionary Should Contain Key    ${status}    stallguard_enabled
    Dictionary Should Contain Key    ${status}    stall_confidence

Test Load Estimation Accuracy
    [Documentation]    Verify load estimation matches simulated load
    [Tags]    adaptive    load    accuracy
    
    Configure Adaptive Features
    
    # Test various load levels
    FOR    ${load}    IN    10    30    50    70    90
        Simulate Load    ${load}
        Sleep    0.2s
        Verify Load Estimation    ${load}    tolerance=10.0
    END

Test Adaptive Control State Persistence
    [Documentation]    Verify adaptive state persists across reconfigurations
    [Tags]    adaptive    state    persistence
    
    # Enable and accumulate energy
    Configure Adaptive Features    coolstep_enable=true
    Simulate Load    ${LOW_LOAD}
    Sleep    1s
    
    ${status1}=    Request Adaptive Status
    ${energy1}=    Get From Dictionary    ${status1}    energy_saved_wh
    
    # Reconfigure (disable and re-enable)
    Configure Adaptive Features    coolstep_enable=false
    Configure Adaptive Features    coolstep_enable=true
    
    Sleep    1s
    ${status2}=    Request Adaptive Status
    ${energy2}=    Get From Dictionary    ${status2}    energy_saved_wh
    
    # Energy should continue accumulating (or reset - depends on implementation)
    Should Be True    ${energy2} >= ${energy1}

# ============================================================================
# Edge Cases and Safety Tests
# ============================================================================

Test Disable All Features
    [Documentation]    Verify system operates normally with all features disabled
    [Tags]    adaptive    safety    edge_case
    
    Configure Adaptive Features
    ...    coolstep_enable=false
    ...    dcstep_enable=false
    ...    stallguard_enable=false
    
    ${status}=    Request Adaptive Status
    ${coolstep_enabled}=    Get From Dictionary    ${status}    coolstep_enabled
    ${dcstep_enabled}=    Get From Dictionary    ${status}    dcstep_enabled
    ${stallguard_enabled}=    Get From Dictionary    ${status}    stallguard_enabled
    
    Should Not Be True    ${coolstep_enabled}
    Should Not Be True    ${dcstep_enabled}
    Should Not Be True    ${stallguard_enabled}
    
    # System should still operate normally
    Send SetTarget V2    target_angle=45.0
    Wait For Motion Complete
    Verify Position Reached    45.0

Test Extreme Load Conditions
    [Documentation]    Verify adaptive control handles extreme loads safely
    [Tags]    adaptive    safety    extreme
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    dcstep_enable=true
    ...    stallguard_enable=true
    
    # Simulate overload (> 100%)
    Simulate Load    120.0
    Sleep    0.5s
    
    ${status}=    Request Adaptive Status
    ${load}=    Get From Dictionary    ${status}    load_percent
    ${velocity_scale}=    Get From Dictionary    ${status}    velocity_scale
    ${stall_status}=    Get From Dictionary    ${status}    stall_status
    
    # System should detect overload and respond
    Should Be True    ${load} > 100.0
    Should Be True    ${velocity_scale} < 1.0
    # May or may not be stalled depending on motion

Test Rapid Load Transitions
    [Documentation]    Verify adaptive control tracks rapid load changes
    [Tags]    adaptive    dynamic    transient
    
    Configure Adaptive Features
    ...    coolstep_enable=true
    ...    dcstep_enable=true
    
    # Rapid load changes
    FOR    ${i}    IN RANGE    5
        Simulate Load    ${LOW_LOAD}
        Sleep    0.1s
        Simulate Load    ${HIGH_LOAD}
        Sleep    0.1s
    END
    
    # Should stabilize at final load
    ${status}=    Request Adaptive Status
    ${current_scale}=    Get From Dictionary    ${status}    current_scale
    ${velocity_scale}=    Get From Dictionary    ${status}    velocity_scale
    
    # Should reflect high load
    Should Be True    ${current_scale} > 0.8
    Should Be True    ${velocity_scale} < 1.0


*** Keywords - Helper Functions ***
# Additional helper functions specific to adaptive control testing

Simulate Normal Motion
    [Arguments]    ${velocity}=30.0    ${current}=1.5
    [Documentation]    Simulate normal motion conditions
    Set Velocity    ${velocity}
    Set Current    ${current}

Simulate Motion
    [Arguments]    ${velocity}    ${current}
    [Documentation]    Simulate motion with specific velocity and current
    Set Velocity    ${velocity}
    Set Current    ${current}

Simulate Blocked Motor
    [Arguments]    ${current}    ${velocity}
    [Documentation]    Simulate blocked/stalled motor
    Set Current    ${current}
    Set Velocity    ${velocity}

Set Load Percentage
    [Arguments]    ${load}
    [Documentation]    Set simulated load percentage
    # Mock implementation - would interface with Renode
    Execute Command    load_sim.set_load(${load})

Set Velocity
    [Arguments]    ${velocity}
    [Documentation]    Set simulated velocity
    Execute Command    motion_sim.set_velocity(${velocity})

Set Current
    [Arguments]    ${current}
    [Documentation]    Set simulated current
    Execute Command    motion_sim.set_current(${current})

Measure FOC Loop Time
    [Documentation]    Measure FOC loop execution time
    ${timing}=    Execute Command    perf.measure_foc_time()
    RETURN    ${timing}

