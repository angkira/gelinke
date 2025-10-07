*** Settings ***
Documentation     Renode-specific helper keywords for hardware emulation tests

Library           String
Library           Collections

# Import Renode keywords from container
Resource          /opt/renode/tests/renode-keywords.robot


*** Variables ***
${FIRMWARE_ELF}        ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware
${TEST_PLATFORM}       ${CURDIR}/../platforms/stm32g431cb.repl
${TEST_SCRIPT}         ${CURDIR}/../scripts/joint_test.resc
${PERIPHERAL_PATH}     ${CURDIR}/../peripherals


*** Keywords ***
# ============================================================================
# Platform Setup & Teardown
# ============================================================================

Setup Renode Platform
    [Documentation]    Initialize Renode with firmware and mock peripherals
    
    # Create machine and load platform
    Execute Command    mach create "joint"
    Execute Command    machine LoadPlatformDescription @${TEST_PLATFORM}
    
    # Load firmware
    Execute Command    sysbus LoadELF @${FIRMWARE_ELF}
    
    # Create CAN hub
    Execute Command    emulation CreateCANHub "can_hub"
    Execute Command    connector Connect sysbus.fdcan1 can_hub
    
    # Setup UART analyzer
    Execute Command    showAnalyzer sysbus.usart1
    
    # Set log level
    Execute Command    logLevel 2

Teardown Renode Platform
    [Documentation]    Clean up Renode emulation
    Execute Command    q

Start Emulation
    [Documentation]    Start Renode emulation
    Execute Command    start

Stop Emulation
    [Documentation]    Pause Renode emulation
    Execute Command    pause

Reset Emulation
    [Documentation]    Reset the emulation
    Execute Command    machine Reset

# ============================================================================
# Mock Peripheral Loading (Python)
# ============================================================================
# Note: These are simplified - real Python peripheral loading in Renode
# requires proper MonitorScript or direct instantiation

Load Mock Encoder
    [Documentation]    Load AS5047P encoder mock (conceptual)
    # In real Renode, Python peripherals are loaded via:
    # - Monitor script: `python "exec(open('peripheral.py').read())"`
    # - Or pre-compiled and loaded as external types
    # For now, this is a placeholder
    Log    Mock encoder would be loaded here    WARN

Load Mock CurrentADC
    [Documentation]    Load current sense ADC mock (conceptual)
    Log    Mock current ADC would be loaded here    WARN

Load Mock CANDevice
    [Documentation]    Load CAN test device mock (conceptual)
    Log    Mock CAN device would be loaded here    WARN

Load Mock Motor
    [Documentation]    Load motor simulator mock (conceptual)
    Log    Mock motor simulator would be loaded here    WARN

# ============================================================================
# Firmware Interaction Keywords
# ============================================================================

Wait For UART Output
    [Arguments]    ${text}    ${timeout}=5s
    [Documentation]    Wait for specific text on UART
    Wait For Line On Uart    ${text}    timeout=${timeout}

Send UART Command
    [Arguments]    ${command}
    [Documentation]    Send command via UART
    Execute Command    sysbus.usart1 WriteString "${command}\\n"

# ============================================================================
# CAN Communication (via registers for now)
# ============================================================================

Send CAN Frame
    [Arguments]    ${can_id}    ${data}
    [Documentation]    Send CAN frame to firmware
    # This would use FDCAN peripheral registers
    # For testing, we can use Execute Command to inject frames
    Log    CAN frame would be sent: ID=0x${can_id:03X}    DEBUG

Wait For CAN Frame
    [Arguments]    ${timeout}=1s
    [Documentation]    Wait for CAN frame from firmware
    # Would read FDCAN registers
    Log    Waiting for CAN frame...    DEBUG

# ============================================================================
# Memory & Register Access
# ============================================================================

Read Memory
    [Arguments]    ${address}    ${size}=4
    [Documentation]    Read memory at address
    ${result}=    Execute Command    sysbus ReadDouble ${address}
    RETURN    ${result}

Write Memory
    [Arguments]    ${address}    ${value}
    [Documentation]    Write value to memory address
    Execute Command    sysbus WriteDouble ${address} ${value}

Read Peripheral Register
    [Arguments]    ${peripheral}    ${offset}
    [Documentation]    Read peripheral register
    ${result}=    Execute Command    ${peripheral} ReadDoubleWord ${offset}
    RETURN    ${result}

Write Peripheral Register
    [Arguments]    ${peripheral}    ${offset}    ${value}
    [Documentation]    Write peripheral register
    Execute Command    ${peripheral} WriteDoubleWord ${offset} ${value}

# ============================================================================
# Time Control
# ============================================================================

Advance Time
    [Arguments]    ${seconds}
    [Documentation]    Advance virtual time by specified seconds
    ${ms}=    Evaluate    int(${seconds} * 1000)
    Execute Command    emulation RunFor "00:00:${seconds}"

Wait Virtual Time
    [Arguments]    ${seconds}
    [Documentation]    Wait for virtual time (same as Advance Time)
    Advance Time    ${seconds}

Get Virtual Time
    [Documentation]    Get current virtual time
    ${time}=    Execute Command    emulation GetTimeSourceInfo
    RETURN    ${time}

# ============================================================================
# Logging & Debugging
# ============================================================================

Enable Debug Logging
    [Documentation]    Enable debug-level logging
    Execute Command    logLevel 1

Enable Info Logging
    [Documentation]    Enable info-level logging
    Execute Command    logLevel 2

Get Logs
    [Documentation]    Get recent log messages
    ${logs}=    Execute Command    log
    RETURN    ${logs}

Take Snapshot
    [Arguments]    ${name}
    [Documentation]    Save emulation snapshot
    Execute Command    Save @${name}.snapshot

Load Snapshot
    [Arguments]    ${name}
    [Documentation]    Load emulation snapshot
    Execute Command    Load @${name}.snapshot

# ============================================================================
# Simplified Test Helpers
# ============================================================================

Quick Setup
    [Documentation]    Quick setup for simple tests
    Setup Renode Platform
    Start Emulation
    Sleep    0.5s    # Let firmware initialize

Quick Teardown
    [Documentation]    Quick cleanup
    Stop Emulation
    Teardown Renode Platform

# ============================================================================
# Assertion Helpers
# ============================================================================

Should Reach State
    [Arguments]    ${expected_state}    ${timeout}=5s
    [Documentation]    Verify firmware reaches expected state
    # Would check via UART output or memory
    Wait For UART Output    ${expected_state}    ${timeout}

Should Be In Range
    [Arguments]    ${value}    ${min}    ${max}
    [Documentation]    Verify value is within range
    ${value_float}=    Convert To Number    ${value}
    ${min_float}=    Convert To Number    ${min}
    ${max_float}=    Convert To Number    ${max}
    
    Should Be True    ${value_float} >= ${min_float}    Value ${value} below minimum ${min}
    Should Be True    ${value_float} <= ${max_float}    Value ${value} above maximum ${max}

