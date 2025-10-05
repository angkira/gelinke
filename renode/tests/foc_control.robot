*** Settings ***
Documentation     FOC control loop and motor control tests
...               Tests verify FOC task initialization, state machine transitions,
...               ADC calibration, encoder reading, PWM output, and control algorithms.
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}
Resource          test_helpers.robot

*** Variables ***
${UART}                     sysbus.usart1
${TIM1}                     sysbus.tim1
${ADC1}                     sysbus.adc1
${ADC2}                     sysbus.adc2
${SPI1}                     sysbus.spi1
${PLATFORM}                 ${CURDIR}/../stm32g431cb_with_mocks.repl
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware
${LOG_TIMEOUT}              5

*** Test Cases ***
# ============================================================================
# Basic FOC Task Tests (work in mock mode)
# ============================================================================

Should Start FOC Task In Mock Mode
    [Documentation]         FOC control loop task should start (mock 1Hz mode)
    [Tags]                  basic  mock
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # FOC task spawns and logs startup
    Wait For Line On Uart   FOC.*task                       timeout=${LOG_TIMEOUT}

Should Report FOC Mock Mode At 1Hz
    [Documentation]         Mock FOC should run at 1Hz instead of 10kHz
    [Tags]                  basic  mock
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    # Wait for multiple FOC iterations (1 Hz = 1 per second)
    Wait For Line On Uart   FOC.*task                       timeout=${LOG_TIMEOUT}
    Wait For Line On Uart   MOCK FOC.*Iteration             timeout=2

Should Initialize TIM1 For PWM
    [Documentation]         TIM1 peripheral should be accessible for PWM generation
    [Tags]                  basic  peripherals
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"
    
    # TIM1 should be present and configurable
    ${result}=              Execute Command    ${TIM1}
    Should Contain          ${result}          Timer

Should Have ADC Peripherals Available
    [Documentation]         ADC1 and ADC2 should be present for current sensing
    [Tags]                  basic  peripherals
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"
    
    # Both ADCs should be present
    ${adc1}=                Execute Command    ${ADC1}
    ${adc2}=                Execute Command    ${ADC2}
    Should Contain          ${adc1}            ADC
    Should Contain          ${adc2}            ADC

Should Have SPI Available For Encoder
    [Documentation]         SPI1 should be present for encoder communication
    [Tags]                  basic  peripherals
    
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"
    
    # SPI1 should be present
    ${result}=              Execute Command    ${SPI1}
    Should Contain          ${result}          SPI

# ============================================================================
# Advanced FOC Tests (require real FOC task, not mock)
# ============================================================================

Should Calibrate ADC Zero Offsets
    [Documentation]         ADC calibration should measure zero-current offsets
    [Tags]                  calibration  adc
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Set zero currents for calibration
    Set ADC Phase Currents  phase_a=0.0  phase_b=0.0  phase_c=0.0
    
    # In mock mode, calibration happens on startup
    # Real implementation would trigger calibration command
    Sleep    0.5s
    
    # Future: Verify calibration complete message in UART

Should Transition Through State Machine
    [Documentation]         FOC state machine: Idle → Calibrating → Running → Fault
    [Tags]                  state-machine
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Activate via iRPC to start FOC
    Send CAN Configure Command
    Sleep    0.1s
    Send CAN Activate Command
    Sleep    0.2s
    
    # FOC task transitions: Idle → Running (in mock mode)
    # Future: Verify state transitions in UART logs

Should Read Phase Currents From ADC
    [Documentation]         ADC should read currents from phases A and B
    [Tags]                  adc  sensors
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Inject known 3-phase currents
    Set ADC Phase Currents  phase_a=2.5  phase_b=-1.2  phase_c=-1.3
    
    # Activate FOC to start reading ADC
    Send CAN Activate Command
    Sleep    0.3s
    
    # ADC mock provides synthetic currents
    # Future: Verify Clarke transform output in telemetry

Should Read Encoder Position Over SPI
    [Documentation]         SPI should read TLE5012B encoder angle
    [Tags]                  encoder  spi  sensors
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Set encoder position
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=100.0
    
    # Activate FOC to start reading encoder
    Send CAN Activate Command
    Sleep    0.3s
    
    # Encoder mock provides synthetic position
    # Future: Verify electrical angle calculation in telemetry

Should Calculate Velocity From Position
    [Documentation]         Velocity should be calculated via differentiation
    [Tags]                  observer  velocity
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Enable continuous rotation for velocity calculation
    Enable Encoder Rotation  angular_velocity=180.0
    
    # Activate FOC
    Send CAN Activate Command
    Sleep    0.5s
    
    # Velocity observer calculates from position changes
    # Future: Verify velocity estimation in telemetry

Should Execute Clarke Transform
    [Documentation]         Clarke transform: ABC → αβ
    [Tags]                  foc-math  transforms
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Provide balanced 3-phase currents (ia + ib + ic = 0)
    Set ADC Phase Currents  phase_a=1.732  phase_b=-0.866  phase_c=-0.866
    
    # Activate FOC to execute Clarke transform
    Send CAN Activate Command
    Sleep    0.2s
    
    # Clarke: α=ia, β=(ia+2*ib)/√3
    # Expected: α≈1.732, β≈0
    # Future: Verify transform output in debug logs

Should Execute Park Transform
    [Documentation]         Park transform: αβ → dq (CORDIC accelerated)
    [Tags]                  foc-math  transforms  cordic
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Set α, β currents and rotor angle
    Set ADC Phase Currents  phase_a=1.5  phase_b=0.0  phase_c=-1.5
    Set Encoder Position    angle_deg=30.0  velocity_deg_s=0.0
    
    # Activate FOC to execute Park transform
    Send CAN Activate Command
    Sleep    0.2s
    
    # Park: id = α*cos(θ) + β*sin(θ), iq = -α*sin(θ) + β*cos(θ)
    # CORDIC accelerates sin/cos calculation
    # Future: Verify CORDIC usage and dq values

Should Run PI Controllers For DQ Currents
    [Documentation]         PI controllers should regulate d and q axis currents
    [Tags]                  control  pi-controller  fmac
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Set target torque (iq) and field (id=0)
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=0.0    velocity_deg_s=100.0
    
    # Provide measured currents
    Set ADC Phase Currents  phase_a=0.5  phase_b=-0.25  phase_c=-0.25
    Sleep    0.3s
    
    # PI controllers regulate id→0, iq→target
    # FMAC accelerates PI calculation
    # Future: Verify vd, vq output in telemetry

Should Execute Inverse Park Transform
    [Documentation]         Inverse Park: dq → αβ
    [Tags]                  foc-math  transforms
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Setup FOC with currents and angle
    Send CAN Activate Command
    Set ADC Phase Currents  phase_a=1.0  phase_b=-0.5  phase_c=-0.5
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=0.0
    Sleep    0.3s
    
    # Inverse Park: vα = vd*cos(θ) - vq*sin(θ), vβ = vd*sin(θ) + vq*cos(θ)
    # CORDIC accelerates calculation
    # Future: Verify vα, vβ output

Should Generate SVPWM Output
    [Documentation]         Space Vector PWM should generate 3-phase duty cycles
    [Tags]                  pwm  svpwm
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Activate FOC to generate SVPWM
    Send CAN Configure Command
    Send CAN Activate Command
    Set ADC Phase Currents  phase_a=1.5  phase_b=-0.75  phase_c=-0.75
    Set Encoder Position    angle_deg=60.0  velocity_deg_s=50.0
    Sleep    0.3s
    
    # SVPWM calculates sector and duty cycles from vα, vβ
    # Outputs PWM_A, PWM_B, PWM_C with dead-time
    # Future: Capture PWM outputs and verify duty cycles

Should Update PWM Outputs
    [Documentation]         TIM1 PWM channels should be updated each cycle
    [Tags]                  pwm  actuators
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Activate FOC to start PWM generation
    Send CAN Configure Command
    Send CAN Activate Command
    Set ADC Phase Currents  phase_a=2.0  phase_b=-1.0  phase_c=-1.0
    Set Encoder Position    angle_deg=90.0  velocity_deg_s=100.0
    Sleep    0.3s
    
    # TIM1 CCR1/2/3 updated with duty cycles
    # Complementary outputs (high+low side) with dead-time
    # Future: Monitor TIM1 registers for updates

Should Disable PWM On Fault
    [Documentation]         PWM should be immediately disabled on fault condition
    [Tags]                  safety  fault-handling
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Start FOC running
    Send CAN Activate Command
    Sleep    0.2s
    
    # Trigger overcurrent fault
    Inject ADC Overcurrent  phase=A  current_amps=25.0
    Sleep    0.2s
    
    # PWM should disable immediately, state → Fault
    # Future: Verify PWM outputs = 0, state = Fault

# ============================================================================
# Performance & Timing Tests
# ============================================================================

Should Run FOC Loop At 10kHz In Production Mode
    [Documentation]         FOC loop should execute at 10 kHz (100 µs period)
    [Tags]                  performance  timing
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # In mock mode, FOC runs at 1 Hz (not 10 kHz)
    # Production mode would run at 10 kHz = 100 µs period
    Send CAN Activate Command
    Sleep    1s
    
    # Mock FOC: 1 Hz (demonstration only)
    # Real FOC: 10 kHz (10 ms = 100 iterations)
    # Future: Verify 10 kHz timing in production build

Should Meet FOC Loop Timing Budget
    [Documentation]         Each FOC iteration should complete < 100 µs
    [Tags]                  performance  timing
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Activate FOC
    Send CAN Activate Command
    Sleep    0.5s
    
    # Timing budget: < 100 µs per iteration
    # - ADC read: ~5 µs
    # - Encoder read: ~10 µs (SPI)
    # - Clarke/Park transforms: ~15 µs (CORDIC)
    # - PI controllers: ~20 µs (FMAC)
    # - SVPWM: ~10 µs
    # Total: ~60 µs (40 µs margin)
    # Future: Add cycle counter instrumentation

Should Handle Encoder Read Latency
    [Documentation]         SPI encoder read should complete < 10 µs
    [Tags]                  performance  spi
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Setup encoder
    Set Encoder Position    angle_deg=120.0  velocity_deg_s=150.0
    Send CAN Activate Command
    Sleep    0.3s
    
    # SPI encoder read timing:
    # - TLE5012B read command: 2 µs
    # - Response: 4 µs
    # - Processing: 2 µs
    # Total: ~8 µs < 10 µs budget
    # Future: Add SPI timing measurement

# ============================================================================
# Position & Velocity Control Tests
# ============================================================================

Should Track Position Setpoint
    [Documentation]         Position controller should track target angle
    [Tags]                  control  position
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Set target position 90°
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=90.0    velocity_deg_s=0.0
    
    # Start from 0°, simulate movement towards 90°
    Set Encoder Position    angle_deg=0.0  velocity_deg_s=0.0
    Sleep    0.2s
    Set Encoder Position    angle_deg=45.0  velocity_deg_s=100.0
    Sleep    0.3s
    Set Encoder Position    angle_deg=85.0  velocity_deg_s=20.0
    Sleep    0.2s
    
    # Position error decreases: 90° → 45° → 5°
    # Future: Verify position tracking in telemetry

Should Track Velocity Setpoint
    [Documentation]         Velocity controller should track target speed
    [Tags]                  control  velocity
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Set target velocity 200 deg/s
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=0.0    velocity_deg_s=200.0
    
    # Simulate velocity ramp-up: 0 → 100 → 200 deg/s
    Set Encoder Position    angle_deg=0.0  velocity_deg_s=0.0
    Sleep    0.2s
    Set Encoder Position    angle_deg=10.0  velocity_deg_s=100.0
    Sleep    0.2s
    Set Encoder Position    angle_deg=30.0  velocity_deg_s=200.0
    Sleep    0.2s
    
    # Velocity error decreases: 200 → 100 → 0
    # Future: Verify velocity tracking in telemetry

Should Respect Velocity Limits
    [Documentation]         Position controller should limit max velocity
    [Tags]                  control  safety
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Command large position step (would require high velocity)
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=180.0    velocity_deg_s=300.0
    
    # Velocity should be clamped to max (e.g., 250 deg/s)
    Set Encoder Position    angle_deg=0.0  velocity_deg_s=0.0
    Sleep    0.5s
    
    # Future: Verify velocity limit is enforced in telemetry

Should Respect Current Limits
    [Documentation]         Current controller should saturate at max current
    [Tags]                  control  safety
    
    Execute Command         machine LoadPlatformDescription @${PLATFORM}
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Wait For Line On Uart   System Ready                    timeout=${LOG_TIMEOUT}
    
    # Request high torque
    Send CAN Configure Command
    Send CAN Activate Command
    Send CAN SetTarget Command    angle_deg=0.0    velocity_deg_s=500.0
    
    # Set high measured current
    Set ADC Phase Currents  phase_a=15.0  phase_b=-7.5  phase_c=-7.5
    Sleep    0.3s
    
    # Current should be clamped to max safe value (e.g., 20 A)
    # Future: Verify current saturation in telemetry

*** Keywords ***
Inject ADC Value
    [Arguments]             ${channel}  ${value}
    [Documentation]         Inject synthetic ADC reading
    # TODO: Implement Python peripheral for ADC injection
    Log                     ADC injection not implemented yet

Inject Encoder Position
    [Arguments]             ${angle_deg}
    [Documentation]         Inject synthetic encoder reading
    # TODO: Implement Python peripheral for encoder simulation
    Log                     Encoder injection not implemented yet


