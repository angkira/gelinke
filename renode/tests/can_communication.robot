*** Settings ***
Documentation     CAN-FD communication tests with iRPC protocol
...               Tests verify CAN initialization, message transmission/reception,
...               iRPC protocol handling, timeout scenarios, and error cases.
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}
Resource          test_helpers.robot

*** Variables ***
${UART}                     sysbus.usart1
${FDCAN}                    sysbus.fdcan1
${PLATFORM}                 ${CURDIR}/../stm32g431cb_with_mocks.repl
${SCRIPT}                   ${CURDIR}/../stm32g431_foc.resc
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware
${LOG_TIMEOUT}              5
${CAN_NODE_ID}              0x0010

# iRPC message bytes (postcard-serialized)
# These are pre-computed from Rust serialization (see tests/irpc_byte_generator.rs)

# Configure: source=0x0000, target=0x0010, msg_id=1, payload=Configure
${IRPC_CONFIGURE}          0x00 0x00 0x10 0x00 0x01 0x00 0x00 0x00 0x01

# Activate: source=0x0000, target=0x0010, msg_id=2, payload=Activate  
${IRPC_ACTIVATE}           0x00 0x00 0x10 0x00 0x02 0x00 0x00 0x00 0x02

# Deactivate: source=0x0000, target=0x0010, msg_id=3, payload=Deactivate
${IRPC_DEACTIVATE}         0x00 0x00 0x10 0x00 0x03 0x00 0x00 0x00 0x03

# Reset: source=0x0000, target=0x0010, msg_id=4, payload=Reset
${IRPC_RESET}              0x00 0x00 0x10 0x00 0x04 0x00 0x00 0x00 0x04

# Invalid message (wrong CRC, malformed)
${IRPC_INVALID}            0xFF 0xFF 0xFF 0xFF 0xFF 0xFF 0xFF 0xFF 0xFF

*** Test Cases ***
Should Initialize FDCAN Peripheral
    [Documentation]         FDCAN1 peripheral should be present and initialized
    [Tags]                  init  basic
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"
    
    # Check FDCAN peripheral is enabled
    ${peripherals}=         Execute Command    sysbus WhatPeripheralsAreEnabled
    Should Contain          ${peripherals}     fdcan1

Should Create CAN Hub For Multi Node Testing
    [Documentation]         CAN hub allows emulation of multi-device CAN network
    [Tags]                  infrastructure
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         emulation CreateCANHub "testBus"
    Execute Command         connector Connect ${FDCAN} testBus
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"

    # Verify hub is created (no error means success)
    ${result}=              Execute Command    connector
    Should Not Contain      ${result}          Error

Should Start CAN Task In Mock Mode
    [Documentation]         CAN task should start even in mock mode
    [Tags]                  basic  mock
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for CAN task to start (mock mode logs this)
    Wait For Line On Uart   CAN task started                timeout=${LOG_TIMEOUT}

Should Handle FDCAN Register Access
    [Documentation]         Firmware should be able to read/write FDCAN registers
    [Tags]                  basic  registers
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.5"
    
    # Check FDCAN registers are accessible
    # Note: This is basic smoke test, real CAN tests need non-mock mode
    ${result}=              Execute Command    ${FDCAN}
    Should Contain          ${result}          MCAN

# ============================================================================
# NOTE: Tests below require NON-MOCK CAN mode!
# Current firmware uses renode-mock feature which disables real CAN task.
# These tests are STUBS for future implementation when:
# 1. Real CAN task can run in Renode, OR
# 2. A hybrid mode is implemented (real CAN + mock FOC)
# ============================================================================

Should Send CAN Frame To Bus
    [Documentation]         Send CAN frame via FDCAN peripheral using mock
    [Tags]                  can-tx
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for system ready
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Send CAN command
    Send CAN Configure Command
    
    # Verify CAN task is processing
    Sleep    0.1s

Should Receive And Process CAN Frame
    [Documentation]         Receive CAN frame and process as iRPC message
    [Tags]                  can-rx  irpc
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for system ready
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Send Configure via mock CAN device
    Send CAN Configure Command
    
    # Note: In current mock mode, firmware uses mock_can task
    # which doesn't actively read FDCAN. This demonstrates the test structure.
    Sleep    0.2s

Should Handle IRPC Configure Command
    [Documentation]         Configure command should transition Unconfigured → Inactive
    [Tags]                  irpc  lifecycle
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for system ready
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Send Configure command via mock
    Send CAN Configure Command
    
    # In mock mode, command is queued in CAN device mock
    # Real implementation would process and transition state
    Sleep    0.2s
    
    # Future: Check CAN Response Received when real CAN task is active

Should Handle IRPC Activate Command
    [Documentation]         Activate command should transition Inactive → Active
    [Tags]                  irpc  lifecycle
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Go through lifecycle: Configure then Activate
    Send CAN Configure Command
    Sleep    0.1s
    Send CAN Activate Command
    Sleep    0.1s
    
    # Commands are queued in mock
    # Future: Verify state transitions when real CAN task active

Should Handle IRPC SetTarget When Active
    [Documentation]         SetTarget should work only in Active state
    [Tags]                  irpc  commands
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Complete lifecycle
    Send CAN Configure Command
    Sleep    0.1s
    Send CAN Activate Command
    Sleep    0.1s
    
    # Now send SetTarget
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Sleep    0.1s
    
    # Commands queued in mock CAN device
    # Future: Verify command processing and response

Should Reject IRPC SetTarget When Inactive
    [Documentation]         SetTarget should fail if not in Active state
    [Tags]                  irpc  commands  negative
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Try SetTarget WITHOUT activating first (should be rejected)
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Sleep    0.1s
    
    # Future: Verify Nack response when real CAN task active

Should Handle CAN Bus Timeout
    [Documentation]         No response within timeout should trigger error
    [Tags]                  irpc  timeout  error-handling
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Send command but don't wait for response
    Send CAN Configure Command
    
    # In real implementation, firmware would timeout waiting for Ack
    # This test demonstrates timeout handling structure
    Sleep    2s
    
    # Future: Verify timeout detection in UART logs

Should Handle Malformed CAN Message
    [Documentation]         Invalid/corrupted message should be rejected
    [Tags]                  irpc  error-handling  negative
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Send invalid/malformed message
    # Note: Direct FDCAN SendFrame not available in current mock
    # This test structure shows how to test error handling
    Sleep    0.2s
    
    # System should remain stable after malformed message
    Wait For Line On Uart   System heartbeat                timeout=2

Should Handle Wrong Node ID Message
    [Documentation]         Messages for different node should be ignored
    [Tags]                  irpc  filtering
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Our node ID is 0x0010
    # Message for different node should be ignored
    # (Mock CAN device currently sends to 0x0010 by default)
    
    # Send valid message to our node
    Send CAN Configure Command
    Sleep    0.1s
    
    # Future: Test filtering with messages to different node IDs

Should Handle CAN Bus Off Error
    [Documentation]         CAN bus-off condition should be detected and handled
    [Tags]                  error-handling  fault
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # CAN bus-off is a hardware error condition
    # In real implementation, would inject CAN errors to trigger bus-off
    # This test structure shows how to verify error detection
    
    # System should continue heartbeat even if CAN errors occur
    Wait For Line On Uart   System heartbeat                timeout=2
    
    # Future: Inject CAN errors and verify recovery

Should Send Periodic Telemetry
    [Documentation]         Firmware should broadcast telemetry at regular intervals
    [Tags]                  telemetry
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Activate joint
    Send CAN Configure Command
    Sleep    0.1s
    Send CAN Activate Command
    Sleep    0.1s
    
    # In real implementation, telemetry would be broadcast periodically
    # Future: Capture and verify telemetry messages on CAN bus
    Sleep    1s

# ============================================================================
# Performance Tests
# ============================================================================

Should Meet CAN Message Latency Requirements
    [Documentation]         CAN message processing should be < 100 µs
    [Tags]                  performance  timing
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Send command
    Send CAN Configure Command
    
    # In Renode, timing is virtual but demonstrates test structure
    # Real implementation would measure actual latency
    Sleep    0.01s
    
    # Future: Add timing instrumentation to measure latency
    # Expected: CAN RX (50µs) + Processing (20µs) + CAN TX (50µs) < 150µs

*** Keywords ***
Send IRPC Message
    [Arguments]             ${message_bytes}
    [Documentation]         Send iRPC message via CAN-FD
    Execute Command         ${FDCAN} SendFrame ${CAN_NODE_ID} ${message_bytes}
    Execute Command         emulation RunFor "00:00:00.001"

Verify IRPC Response
    [Arguments]             ${expected_payload}
    [Documentation]         Verify iRPC response was received
    # TODO: Implement CAN frame capture and verification
    Log                     Response verification not implemented yet

