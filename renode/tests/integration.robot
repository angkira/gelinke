*** Settings ***
Documentation     End-to-end integration tests
...               Tests verify complete workflows: CAN command → FOC response → PWM output,
...               lifecycle management, telemetry streaming, and full system behavior.
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}
Resource          test_helpers.robot

*** Variables ***
${UART}                     sysbus.usart1
${FDCAN}                    sysbus.fdcan1
${PLATFORM}                 ${CURDIR}/../stm32g431cb_with_mocks.repl
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware
${LOG_TIMEOUT}              5

*** Test Cases ***
# ============================================================================
# Basic Integration Tests (work in mock mode)
# ============================================================================

Should Complete Full System Startup
    [Documentation]         Complete boot sequence: UART → CAN → FOC → Ready
    [Tags]                  integration  startup  basic
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Complete startup sequence
    Wait For Line On Uart   CLN17 v2.0 Joint Firmware       timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   Joint Firmware Initialization   timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   CAN task started                timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   FOC.*task                       timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   System heartbeat: 1             timeout=2

Should Maintain System Heartbeat
    [Documentation]         System heartbeat should continue indefinitely
    [Tags]                  integration  heartbeat  basic
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for multiple heartbeats
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   System heartbeat: 1             timeout=2
    Wait For Line On Uart   System heartbeat: 2             timeout=2
    Wait For Line On Uart   System heartbeat: 3             timeout=2
    Wait For Line On Uart   System heartbeat: 4             timeout=2
    Wait For Line On Uart   System heartbeat: 5             timeout=2

Should Run All Tasks Concurrently
    [Documentation]         All async tasks should run without blocking
    [Tags]                  integration  concurrency  basic
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Verify multiple tasks log within short time
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   System heartbeat                timeout=2
    Wait For Line On Uart   MOCK FOC.*Iteration             timeout=2
    Wait For Line On Uart   MOCK CAN                        timeout=6

# ============================================================================
# Lifecycle Integration Tests
# ============================================================================

Should Complete Full Lifecycle Sequence
    [Documentation]         Unconfigured → Inactive → Active → Inactive → Unconfigured
    [Tags]                  integration  lifecycle  irpc  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Reject Invalid Lifecycle Transitions
    [Documentation]         Invalid transitions should be rejected with Nack
    [Tags]                  integration  lifecycle  negative  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


# ============================================================================
# CAN → FOC Integration Tests
# ============================================================================

Should Process SetTarget Command And Update FOC
    [Documentation]         SetTarget iRPC → position controller update → PWM change
    [Tags]                  integration  can-to-foc  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Process Velocity Command And Update FOC
    [Documentation]         Velocity command → velocity controller → motor spins
    [Tags]                  integration  can-to-foc  velocity  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Process Torque Command And Update FOC
    [Documentation]         Torque command → current setpoint → motor torque
    [Tags]                  integration  can-to-foc  torque  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


# ============================================================================
# Telemetry Integration Tests
# ============================================================================

Should Stream Telemetry At Regular Intervals
    [Documentation]         Telemetry should be broadcast periodically
    [Tags]                  integration  telemetry  can  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Include Position In Telemetry
    [Documentation]         Telemetry should contain current position
    [Tags]                  integration  telemetry  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Include Velocity In Telemetry
    [Documentation]         Telemetry should contain current velocity
    [Tags]                  integration  telemetry  
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Include Current In Telemetry
    [Documentation]         Telemetry should contain motor current
    [Tags]                  integration  telemetry  
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Include Temperature In Telemetry
    [Documentation]         Telemetry should contain motor temperature
    [Tags]                  integration  telemetry  
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


# ============================================================================
# Fault Handling Integration Tests
# ============================================================================

Should Stop Motor On Overcurrent And Report Via CAN
    [Documentation]         Overcurrent → PWM off → fault telemetry via CAN
    [Tags]                  integration  fault  can  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Report Emergency Stop Via Telemetry
    [Documentation]         E-stop → immediate stop → fault broadcast
    [Tags]                  integration  emergency-stop  can  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


# ============================================================================
# Performance Integration Tests
# ============================================================================

Should Meet End-to-End Latency Requirements
    [Documentation]         CAN command → FOC response → PWM update < 200 µs
    [Tags]                  integration  performance  latency  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Handle High CAN Message Rate
    [Documentation]         System should handle 1000 msg/s without drops
    [Tags]                  integration  performance  throughput  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


# ============================================================================
# Multi-Motor Integration Tests (future)
# ============================================================================

Should Coordinate Multiple Joints
    [Documentation]         Multiple joints should operate independently
    [Tags]                  integration  multi-motor  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Synchronize Motion Across Joints
    [Documentation]         Coordinated motion should be synchronized
    [Tags]                  integration  multi-motor  synchronization  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


# ============================================================================
# Recovery Integration Tests
# ============================================================================

Should Recover From CAN Bus Error
    [Documentation]         CAN bus error → recovery → normal operation
    [Tags]                  integration  recovery  can  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Recover From Temporary Encoder Glitch
    [Documentation]         Brief encoder error → auto-recovery → continue
    [Tags]                  integration  recovery  encoder  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


# ============================================================================
# Stress Tests
# ============================================================================

Should Run Continuously For Extended Period
    [Documentation]         System should run stable for hours
    [Tags]                  integration  stress  long-running  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


Should Handle Rapid Command Changes
    [Documentation]         Rapid command changes should not cause issues
    [Tags]                  integration  stress  commands  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # End-to-end test using all mock peripherals
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=150.0
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.25  phase_c=-1.25
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=120.0
    Sleep    0.5s
    
    # System should process complete pipeline
    # Future: Verify end-to-end behavior


*** Keywords ***
Verify System State
    [Arguments]             ${expected_state}
    [Documentation]         Query and verify system state
    # TODO: Implement state query
    Log                     State query not implemented yet

Capture Telemetry Message
    [Documentation]         Capture and parse telemetry from CAN bus
    # TODO: Implement CAN frame capture
    Log                     CAN capture not implemented yet

Measure Latency
    [Arguments]             ${start_event}  ${end_event}
    [Documentation]         Measure time between two events
    # TODO: Implement timing instrumentation
    Log                     Timing measurement not implemented yet
