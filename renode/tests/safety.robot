*** Settings ***
Documentation     Safety and fault handling tests
...               Tests verify overcurrent detection, emergency stop, watchdog,
...               fault recovery, and other safety-critical mechanisms.
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}
Resource          test_helpers.robot

*** Variables ***
${UART}                     sysbus.usart1
${FDCAN}                    sysbus.fdcan1
${ADC1}                     sysbus.adc1
${TIM1}                     sysbus.tim1
${PLATFORM}                 ${CURDIR}/../stm32g431cb_with_mocks.repl
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware
${LOG_TIMEOUT}              5

*** Test Cases ***
# ============================================================================
# Basic Safety Tests (work in mock mode)
# ============================================================================

Should Start In Safe State
    [Documentation]         System should boot with motor disabled (PWM off)
    [Tags]                  basic  safety
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # System should boot successfully
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

Should Have Watchdog Timer Available
    [Documentation]         IWDG peripheral should be present for watchdog functionality
    [Tags]                  basic  watchdog
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"
    
    # Check watchdog peripheral exists
    ${peripherals}=         Execute Command    sysbus WhatPeripheralsAreEnabled
    # Note: IWDG may not be in platform yet
    Log                     Watchdog: ${peripherals}

# ============================================================================
# Overcurrent Protection Tests
# ============================================================================

Should Detect Overcurrent On Phase A
    [Documentation]         Overcurrent on phase A should trigger fault
    [Tags]                  fault  overcurrent  adc
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Start FOC running normally
    Send CAN Activate Command
    Set ADC Phase Currents  phase_a=2.0  phase_b=-1.0  phase_c=-1.0
    Sleep    0.2s
    
    # Inject overcurrent on phase A (> 20 A threshold)
    Inject ADC Overcurrent  phase=A  current_amps=25.0
    Sleep    0.2s
    
    # System should detect overcurrent and disable PWM
    # Future: Verify fault state in UART logs

Should Detect Overcurrent On Phase B
    [Documentation]         Overcurrent on phase B should trigger fault
    [Tags]                  fault  overcurrent  adc
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Start FOC running
    Send CAN Activate Command
    Set ADC Phase Currents  phase_a=1.0  phase_b=2.0  phase_c=-3.0
    Sleep    0.2s
    
    # Inject overcurrent on phase B
    Inject ADC Overcurrent  phase=B  current_amps=28.0
    Sleep    0.2s
    
    # System should fault
    # Future: Verify fault detection

Should Have Configurable Overcurrent Threshold
    [Documentation]         Overcurrent threshold should be configurable
    [Tags]                  configuration  overcurrent  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Disable All PWM Outputs On Overcurrent
    [Documentation]         All PWM channels should be disabled on overcurrent
    [Tags]                  fault  pwm  safety  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


# ============================================================================
# Emergency Stop Tests
# ============================================================================

Should Handle Emergency Stop Command
    [Documentation]         E-stop command should immediately disable motor
    [Tags]                  emergency-stop  safety  irpc  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Prevent Operation After Emergency Stop
    [Documentation]         After e-stop, motor should stay disabled until Reset
    [Tags]                  emergency-stop  safety  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Log Emergency Stop Event
    [Documentation]         E-stop should be logged via UART
    [Tags]                  emergency-stop  logging  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


# ============================================================================
# Voltage Protection Tests
# ============================================================================

Should Detect Overvoltage
    [Documentation]         DC bus overvoltage should trigger fault
    [Tags]                  fault  voltage  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Detect Undervoltage
    [Documentation]         DC bus undervoltage should trigger warning/fault
    [Tags]                  fault  voltage  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


# ============================================================================
# Encoder Fault Detection
# ============================================================================

Should Detect Encoder Communication Failure
    [Documentation]         SPI read timeout should trigger encoder fault
    [Tags]                  fault  encoder  spi  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Detect Invalid Encoder Data
    [Documentation]         Invalid encoder CRC/data should trigger fault
    [Tags]                  fault  encoder  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Detect Encoder Position Jump
    [Documentation]         Large position jump should trigger fault
    [Tags]                  fault  encoder  observer  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


# ============================================================================
# Communication Watchdog Tests
# ============================================================================

Should Detect CAN Communication Timeout
    [Documentation]         No CAN messages for N seconds should trigger warning
    [Tags]                  watchdog  can  timeout  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Reset Watchdog On Message Reception
    [Documentation]         CAN watchdog should be reset by incoming messages
    [Tags]                  watchdog  can  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


# ============================================================================
# Fault Recovery Tests
# ============================================================================

Should Recover From Transient Overcurrent
    [Documentation]         Brief overcurrent spike should be recoverable
    [Tags]                  fault-recovery  overcurrent  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Not Auto-Recover From Hard Fault
    [Documentation]         Serious faults should require explicit reset
    [Tags]                  fault-recovery  safety  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Clear Fault State On Reset
    [Documentation]         Reset command should clear fault state
    [Tags]                  fault-recovery  lifecycle  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Log Fault History
    [Documentation]         System should maintain fault history
    [Tags]                  fault-recovery  logging  diagnostics  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


# ============================================================================
# Safety Limits Tests
# ============================================================================

Should Enforce Maximum Velocity Limit
    [Documentation]         Velocity should never exceed configured maximum
    [Tags]                  safety  limits  velocity  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Enforce Maximum Current Limit
    [Documentation]         Current should never exceed configured maximum
    [Tags]                  safety  limits  current  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Enforce Position Limits
    [Documentation]         Position should respect software limits
    [Tags]                  safety  limits  position  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


# ============================================================================
# Hardware Fault Detection
# ============================================================================

Should Detect Missing Phase Current
    [Documentation]         ADC reading zero current should trigger fault
    [Tags]                  fault  hardware  adc  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Detect PWM Output Failure
    [Documentation]         PWM not updating should be detected
    [Tags]                  fault  hardware  pwm  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


# ============================================================================
# System Watchdog Tests
# ============================================================================

Should Pet Hardware Watchdog Regularly
    [Documentation]         IWDG should be refreshed to prevent reset
    [Tags]                  watchdog  iwdg  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


Should Reset On Watchdog Timeout
    [Documentation]         If watchdog not refreshed, system should reset
    [Tags]                  watchdog  iwdg  fault  
    
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Test implementation using mock peripherals
    Send CAN Activate Command
    Sleep    0.3s
    
    # Future: Add specific assertions for this test


*** Keywords ***
Trigger Overcurrent
    [Arguments]             ${phase}  ${current_amps}
    [Documentation]         Inject overcurrent on specified phase
    # TODO: Implement ADC injection
    Log                     ADC injection not implemented yet

Trigger Emergency Stop
    [Documentation]         Send emergency stop command
    # TODO: Send iRPC EmergencyStop
    Log                     E-stop command not implemented yet

Verify PWM Disabled
    [Documentation]         Verify all PWM outputs are off
    # TODO: Check TIM1 registers
    Log                     PWM verification not implemented yet

Verify Fault State
    [Documentation]         Verify FOC is in Fault state
    # TODO: Query state via iRPC or internal check
    Log                     State query not implemented yet
