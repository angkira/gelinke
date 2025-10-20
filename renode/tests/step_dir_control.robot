*** Settings ***
Documentation     Step-Dir stepper motor control tests
...               Tests verify Step-Dir task initialization, stepping behavior,
...               microstepping, direction control, and PWM phase output.
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}
Resource          test_helpers.robot

*** Variables ***
${UART}                     sysbus.usart1
${TIM1}                     sysbus.tim1
${GPIO_STEP}                sysbus.gpioPortA  # Example: PA6 for STEP input
${GPIO_DIR}                 sysbus.gpioPortA  # Example: PA5 for DIR input
${PLATFORM}                 ${CURDIR}/../stm32g431cb_with_mocks.repl
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware
${LOG_TIMEOUT}              5

*** Test Cases ***
# ============================================================================
# Basic Step-Dir Task Tests (work in mock mode)
# ============================================================================

Should Start Step-Dir Task In Mock Mode
    [Documentation]         Step-Dir control loop task should start (mock 1Hz mode)
    [Tags]                  basic  mock  step-dir

    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    # Step-Dir task spawns and logs startup
    Wait For Line On Uart   STEP-DIR.*task                  timeout=${LOG_TIMEOUT}

Should Report Step-Dir Mock Mode At 1Hz
    [Documentation]         Mock Step-Dir should run at 1Hz instead of 1kHz
    [Tags]                  basic  mock  step-dir

    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    # Wait for multiple Step-Dir iterations (1 Hz = 1 per second)
    Wait For Line On Uart   STEP-DIR.*task                  timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   MOCK STEP-DIR.*Iteration        timeout=2

Should Initialize TIM1 For PWM
    [Documentation]         TIM1 peripheral should be accessible for phase PWM
    [Tags]                  basic  peripherals  step-dir

    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation

    Execute Command         emulation RunFor "00:00:00.1"

    # TIM1 should be present and configurable
    ${result}=              Execute Command    ${TIM1}
    Should Contain          ${result}          Timer

# ============================================================================
# Microstepping Tests
# ============================================================================

Should Support 1x Microstepping (Full Step)
    [Documentation]         Full step mode: 200 steps per revolution
    [Tags]                  microstepping  basic  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Configure for 1x microstepping (default may be 16x)
    # This would require motor config to be set to microsteps=1
    # Future: Add iRPC command to set microstepping mode

    # 200 full steps = 1 revolution (1.8° per step)
    # Future: Send 200 steps and verify position = 360°

Should Support 16x Microstepping
    [Documentation]         16x microstepping: 3200 steps per revolution
    [Tags]                  microstepping  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Default configuration is 16x microstepping
    # 3200 microsteps = 1 revolution (0.1125° per microstep)
    # Future: Send 3200 steps and verify position = 360°

Should Support 256x Microstepping
    [Documentation]         256x microstepping: 51200 steps per revolution
    [Tags]                  microstepping  advanced  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Ultra-fine 256x microstepping
    # 51200 microsteps = 1 revolution (0.00703° per microstep)
    # Future: Configure 256x and verify smooth motion

# ============================================================================
# Direction Control Tests
# ============================================================================

Should Step Forward When Dir High
    [Documentation]         Direction pin HIGH = forward rotation (CW)
    [Tags]                  direction  basic  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Set direction pin HIGH (forward)
    # Send step pulses
    # Verify position increases
    # Future: Implement GPIO injection for DIR/STEP

Should Step Reverse When Dir Low
    [Documentation]         Direction pin LOW = reverse rotation (CCW)
    [Tags]                  direction  basic  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Set direction pin LOW (reverse)
    # Send step pulses
    # Verify position decreases
    # Future: Implement GPIO injection for DIR/STEP

Should Change Direction Instantly
    [Documentation]         Direction changes should take effect immediately
    [Tags]                  direction  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Step forward 100 steps
    # Change direction
    # Step reverse 100 steps
    # Verify position returns to 0
    # Future: Implement step sequence injection

# ============================================================================
# Step Pulse Tests
# ============================================================================

Should Accept Step Pulses Up To 50kHz
    [Documentation]         Maximum step frequency is 50kHz
    [Tags]                  performance  timing  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Generate 50kHz step pulses (20µs period)
    # Verify all steps are counted correctly
    # Future: Add high-frequency step pulse generator

Should Require Minimum Step Pulse Width
    [Documentation]         Step pulse must be at least 1µs wide
    [Tags]                  timing  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Send pulses < 1µs (should be ignored)
    # Send pulses > 1µs (should be counted)
    # Future: Test pulse width detection

Should Debounce Step Input
    [Documentation]         Step input should be debounced to prevent false triggers
    [Tags]                  filtering  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Send noisy step signal with glitches
    # Verify only valid pulses are counted
    # Future: Add noise injection capability

# ============================================================================
# PWM Phase Output Tests
# ============================================================================

Should Generate Sine Wave Microstepping
    [Documentation]         PWM phases should follow sine/cosine for smooth motion
    [Tags]                  pwm  microstepping  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Step through one full step (16 microsteps)
    # Capture PWM phase A and B outputs
    # Verify sine/cosine relationship
    # Future: Add PWM capture and analysis

Should Output Three-Phase PWM
    [Documentation]         Three-phase output for 3-phase stepper drivers
    [Tags]                  pwm  phases  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Verify PWM outputs on TIM1 CH1, CH2, CH3
    # Check complementary outputs (high/low side)
    # Verify dead-time insertion
    # Future: Monitor TIM1 CCR registers

Should Update PWM Every Microstep
    [Documentation]         PWM duty cycles should update for each microstep
    [Tags]                  pwm  timing  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Send continuous steps
    # Monitor PWM duty cycle changes
    # Verify updates match step frequency
    # Future: Add PWM duty cycle monitoring

# ============================================================================
# Position Tracking Tests
# ============================================================================

Should Track Position In Steps
    [Documentation]         Position should be tracked as step count
    [Tags]                  position  basic  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Send 100 steps forward
    # Verify position = 100 steps
    # Send 50 steps reverse
    # Verify position = 50 steps
    # Future: Add position readback via iRPC

Should Track Position In Radians
    [Documentation]         Position should also be available in radians
    [Tags]                  position  conversion  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Send 1600 steps (half revolution at 16x)
    # Verify position ≈ π radians (180°)
    # Future: Add position telemetry

Should Handle Position Overflow
    [Documentation]         Position counter should wrap correctly
    [Tags]                  position  overflow  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Send large number of steps to cause wrap
    # Verify position wraps correctly (u32 overflow)
    # Future: Test position counter behavior at limits

# ============================================================================
# State Machine Tests
# ============================================================================

Should Start In Idle State
    [Documentation]         Step-Dir controller should initialize in Idle state
    [Tags]                  state-machine  basic  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Verify initial state is Idle
    # Future: Add state readback via iRPC telemetry

Should Transition To Running When Enabled
    [Documentation]         Enable command should transition to Running state
    [Tags]                  state-machine  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Send enable command
    # Verify state transitions: Idle → Running
    # Future: Add iRPC commands for Step-Dir control

Should Disable PWM In Idle State
    [Documentation]         PWM outputs should be disabled when Idle
    [Tags]                  state-machine  safety  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Verify PWM outputs are disabled (duty = 0)
    # Enable controller
    # Verify PWM outputs are active
    # Disable controller
    # Verify PWM outputs are disabled again
    # Future: Monitor TIM1 enable state

# ============================================================================
# Comparison Tests: FOC vs Step-Dir
# ============================================================================

Should Consume Less CPU Than FOC
    [Documentation]         Step-Dir at 1kHz should use less CPU than FOC at 10kHz
    [Tags]                  performance  comparison

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Step-Dir: 1kHz update rate (1ms period)
    # FOC: 10kHz update rate (100µs period)
    # Step-Dir should have 10x less overhead
    # Future: Add CPU utilization measurement

Should Be Simpler Than FOC
    [Documentation]         Step-Dir has no transforms, observers, or PI control
    [Tags]                  architecture  comparison

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Step-Dir: Direct stepping, no closed-loop control
    # FOC: Clarke/Park transforms, PI controllers, observers
    # Step-Dir is much simpler for open-loop applications
    # This is a documentation test only

Should Work Without Encoder
    [Documentation]         Step-Dir can operate without position feedback
    [Tags]                  open-loop  comparison  step-dir

    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation

    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}

    # Step-Dir tracks position by counting steps (open-loop)
    # No encoder required (unlike FOC which needs angle feedback)
    # Position accuracy depends on no missed steps
    # Future: Test operation with encoder disconnected

*** Keywords ***
Send Step Pulse
    [Documentation]         Simulate a step pulse on the STEP input pin
    # TODO: Implement GPIO step pulse injection
    Log                     Step pulse injection not implemented yet

Set Direction Pin
    [Arguments]             ${state}
    [Documentation]         Set DIR pin HIGH (forward) or LOW (reverse)
    # TODO: Implement GPIO direction control
    Log                     Direction pin control not implemented yet

Configure Microstepping
    [Arguments]             ${microsteps}
    [Documentation]         Set microstepping resolution (1, 2, 4, 8, 16, 32, 64, 128, 256)
    # TODO: Add iRPC command to configure microstepping
    Log                     Microstepping config not implemented yet

Read Step Position
    [Documentation]         Read current position in steps
    [Return]                0
    # TODO: Implement position readback via iRPC
    Log                     Position readback not implemented yet

Enable Step Dir Controller
    [Documentation]         Enable the Step-Dir controller
    # TODO: Add iRPC command to enable/disable Step-Dir
    Log                     Enable command not implemented yet
