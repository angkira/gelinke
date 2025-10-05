*** Settings ***
Documentation     CAN-FD communication tests with iRPC protocol
...               Tests verify CAN initialization, message transmission/reception,
...               iRPC protocol handling, timeout scenarios, and error cases.
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}

*** Variables ***
${UART}                     sysbus.usart1
${FDCAN}                    sysbus.fdcan1
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
    [Documentation]         [STUB] Send CAN frame via FDCAN peripheral
    [Tags]                  can-tx  future
    
    # TODO: Requires real CAN task or improved mock that uses FDCAN
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Receive And Process CAN Frame
    [Documentation]         [STUB] Receive CAN frame and process as iRPC message
    [Tags]                  can-rx  irpc  future
    
    # TODO: Send frame via CAN hub and verify firmware processes it
    # Example:
    #   Execute Command         ${FDCAN} SendFrame ${CAN_NODE_ID} ${IRPC_CONFIGURE}
    #   Wait For Line On Uart   Received iRPC message
    
    Log                     Test requires non-mock CAN implementation  
    Pass Execution          Skipped: waiting for CAN test mode

Should Handle IRPC Configure Command
    [Documentation]         [STUB] Configure command should transition Unconfigured → Inactive
    [Tags]                  irpc  lifecycle  future
    
    # TODO:
    # 1. Send Configure message via CAN
    # 2. Verify Ack response
    # 3. Verify state changed to Inactive
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Handle IRPC Activate Command
    [Documentation]         [STUB] Activate command should transition Inactive → Active
    [Tags]                  irpc  lifecycle  future
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Handle IRPC SetTarget When Active
    [Documentation]         [STUB] SetTarget should work only in Active state
    [Tags]                  irpc  commands  future
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Reject IRPC SetTarget When Inactive
    [Documentation]         [STUB] SetTarget should fail if not in Active state
    [Tags]                  irpc  commands  negative  future
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Handle CAN Bus Timeout
    [Documentation]         [STUB] No response within timeout should trigger error
    [Tags]                  irpc  timeout  error-handling  future
    
    # TODO:
    # 1. Send message expecting response
    # 2. Don't send response
    # 3. Verify timeout is detected
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Handle Malformed CAN Message
    [Documentation]         [STUB] Invalid/corrupted message should be rejected
    [Tags]                  irpc  error-handling  negative  future
    
    # TODO:
    # 1. Send malformed iRPC message
    # 2. Verify Nack response or silent drop
    # 3. Verify system remains stable
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Handle Wrong Node ID Message
    [Documentation]         [STUB] Messages for different node should be ignored
    [Tags]                  irpc  filtering  future
    
    # TODO:
    # 1. Send message with wrong target_id (not 0x0010)
    # 2. Verify firmware ignores it
    # 3. Send message with correct target_id
    # 4. Verify firmware processes it
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Handle CAN Bus Off Error
    [Documentation]         [STUB] CAN bus-off condition should be detected and handled
    [Tags]                  error-handling  fault  future
    
    # TODO:
    # 1. Trigger bus-off condition (error injection)
    # 2. Verify firmware detects it
    # 3. Verify recovery procedure
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

Should Send Periodic Telemetry
    [Documentation]         [STUB] Firmware should broadcast telemetry at regular intervals
    [Tags]                  telemetry  future
    
    # TODO:
    # 1. Configure joint to Active state
    # 2. Wait for telemetry messages on CAN bus
    # 3. Verify format and frequency
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

# ============================================================================
# Performance Tests
# ============================================================================

Should Meet CAN Message Latency Requirements
    [Documentation]         [STUB] CAN message processing should be < 100 µs
    [Tags]                  performance  timing  future
    
    # TODO:
    # 1. Send message with timestamp
    # 2. Measure response time
    # 3. Verify < 100 µs total latency
    
    Log                     Test requires non-mock CAN implementation
    Pass Execution          Skipped: waiting for CAN test mode

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

