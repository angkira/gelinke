*** Settings ***
Documentation     Example test demonstrating Python mock peripherals usage
...               This test shows how to use CAN, ADC, and Encoder mocks
...               to test firmware WITHOUT code changes!
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}
Resource          test_helpers.robot

*** Variables ***
${UART}                     sysbus.usart1
${PLATFORM}                 ${CURDIR}/../stm32g431cb_with_mocks.repl
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware
${LOG_TIMEOUT}              5

*** Test Cases ***
Example 1: Basic Mock Usage
    [Documentation]         Simple test showing mock peripheral access
    [Tags]                  example  basic
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for system ready
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Set ADC currents
    Set ADC Phase Current    A    2.0
    Set ADC Phase Current    B    -1.0
    Set ADC Phase Current    C    -1.0
    
    # Set encoder angle
    Set Encoder Angle    45.0
    
    # Verify mocks are working
    ${angle}=    Read Encoder Angle
    Should Be True    ${angle} > 40.0 and ${angle} < 50.0
    
    # Check ADC values
    ${current_a}=    Read ADC Phase Current    A
    Should Be True    ${current_a} > 2000
    
    Log    ✅ Mocks are working!

Example 2: ADC Synthetic Motion
    [Documentation]         Test ADC mock with synthetic 3-phase currents
    [Tags]                  example  adc
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Enable synthetic motion (simulates motor running)
    Enable ADC Synthetic Motion    velocity_rad_s=1.0    amplitude_amps=2.0
    
    # Let it run for a bit
    Sleep    0.5s
    
    # Read currents - they should be changing (sinusoidal)
    ${current_a_1}=    Read ADC Phase Current    A
    Sleep    0.1s
    ${current_a_2}=    Read ADC Phase Current    A
    
    # Values should be different (motion is happening)
    Should Not Be Equal    ${current_a_1}    ${current_a_2}
    
    Log    ✅ Synthetic motion working!

Example 3: Encoder Motion
    [Documentation]         Test encoder mock with continuous rotation
    [Tags]                  example  encoder
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Start at 0 degrees
    Set Encoder Angle    0.0
    ${angle_start}=    Read Encoder Angle
    
    # Enable rotation at 30 degrees/second
    Enable Encoder Motion    velocity_deg_s=30.0
    
    # Wait 2 seconds
    Sleep    2s
    
    # Angle should have changed by ~60 degrees
    ${angle_end}=    Read Encoder Angle
    ${angle_change}=    Evaluate    ${angle_end} - ${angle_start}
    
    # Allow some tolerance
    Should Be True    ${angle_change} > 50.0
    Should Be True    ${angle_change} < 70.0
    
    Log    ✅ Encoder rotation working! Changed by ${angle_change}°

Example 4: Overcurrent Injection
    [Documentation]         Test fault handling with ADC overcurrent injection
    [Tags]                  example  fault
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Setup running motor conditions
    Setup Running Motor Conditions    velocity_deg_s=30.0    current_amps=2.0
    
    # Let it run normally
    Sleep    1s
    
    # Inject overcurrent (20A)
    Inject ADC Overcurrent    phase=A
    
    # Wait a bit for firmware to detect
    Sleep    0.5s
    
    # Note: In current mock mode, firmware may not actively monitor ADC
    # This is an example of what WOULD happen with real FOC task
    Log    ✅ Overcurrent injected (firmware detection depends on FOC task)

Example 5: Complete Scenario
    [Documentation]         Full scenario with all mocks working together
    [Tags]                  example  integration
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for startup
    Wait For Line On Uart   CLN17 v2.0 Joint Firmware       timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Setup nominal conditions
    Setup Nominal Operating Conditions
    Log    ✅ Step 1: Nominal conditions set
    
    # Verify encoder at zero
    ${angle}=    Read Encoder Angle
    Should Be True    ${angle} < 5.0
    Log    ✅ Step 2: Encoder at 0° (actual: ${angle}°)
    
    # Verify currents at zero
    ${current_a}=    Read ADC Phase Current    A
    # Should be near offset (2048)
    Should Be True    ${current_a} > 2000 and ${current_a} < 2100
    Log    ✅ Step 3: ADC at zero current (raw: ${current_a})
    
    # Start motor simulation
    Setup Running Motor Conditions    velocity_deg_s=45.0    current_amps=1.5
    Log    ✅ Step 4: Motor simulation started
    
    # Wait for motion
    Sleep    2s
    
    # Check encoder has moved
    ${angle_after}=    Read Encoder Angle
    Should Be True    ${angle_after} > 80.0
    Log    ✅ Step 5: Encoder moved to ${angle_after}°
    
    # Check currents are changing
    ${current_a_now}=    Read ADC Phase Current    A
    Should Not Be Equal    ${current_a}    ${current_a_now}
    Log    ✅ Step 6: ADC currents changing (was: ${current_a}, now: ${current_a_now})
    
    # Stop motion
    Disable ADC Synthetic Motion
    Disable Encoder Motion
    Log    ✅ Step 7: Motion stopped
    
    Log    ✅✅✅ Complete scenario SUCCESS! All mocks working!

*** Keywords ***
# Add any custom keywords here if needed
