*** Settings ***
Documentation     Helper keywords for controlling mock peripherals
Library           String

*** Keywords ***
# ============================================================================
# CAN Device Mock Control
# ============================================================================

Send CAN Configure Command
    [Documentation]    Send iRPC Configure command from external CAN device
    # Write to CAN device mock to queue Configure command
    Execute Command    sysbus.canDeviceMock WriteDoubleWord 0x00 0x01
    # Trigger send via CAN hub (implementation-specific)
    Execute Command    emulation RunFor "00:00:00.001"

Send CAN Activate Command
    [Documentation]    Send iRPC Activate command
    Execute Command    sysbus.canDeviceMock WriteDoubleWord 0x00 0x02
    Execute Command    emulation RunFor "00:00:00.001"

Send CAN SetTarget Command
    [Arguments]    ${angle_deg}  ${velocity_deg_s}
    [Documentation]    Send iRPC SetTarget command with angle and velocity
    # This is simplified - real implementation would encode floats
    Execute Command    sysbus.canDeviceMock WriteDoubleWord 0x00 0x03
    Execute Command    emulation RunFor "00:00:00.001"

Check CAN Response Received
    [Documentation]    Check if CAN response was received from firmware
    ${result}=    Execute Command    sysbus.canDeviceMock ReadDoubleWord 0x00
    Should Not Be Equal    ${result}    0

# ============================================================================
# ADC Mock Control
# ============================================================================

Set ADC Phase Current
    [Arguments]    ${phase}  ${current_amps}
    [Documentation]    Set phase current in Amperes
    ...               phase: A (0x00), B (0x04), C (0x08)
    ...               current_amps: current value (can be negative)
    
    # Convert to ADC raw value (offset 2048, ~200 counts per Amp)
    ${offset}=    Set Variable    2048
    ${counts}=    Evaluate    int(${current_amps} * 200)
    ${raw_value}=    Evaluate    ${offset} + ${counts}
    
    # Clamp to 12-bit range
    ${raw_value}=    Evaluate    max(0, min(4095, ${raw_value}))
    
    # Determine register offset
    ${reg_offset}=    Set Variable If
    ...    '${phase}' == 'A'    0x00
    ...    '${phase}' == 'B'    0x04
    ...    '${phase}' == 'C'    0x08
    
    Execute Command    sysbus.adcMock WriteDoubleWord ${reg_offset} ${raw_value}

Set ADC DC Voltage
    [Arguments]    ${voltage_v}
    [Documentation]    Set DC bus voltage in Volts (0-60V)
    
    # Convert to 12-bit ADC value
    ${adc_value}=    Evaluate    int((${voltage_v} / 60.0) * 4095)
    ${adc_value}=    Evaluate    max(0, min(4095, ${adc_value}))
    
    Execute Command    sysbus.adcMock WriteDoubleWord 0x0C ${adc_value}

Enable ADC Synthetic Motion
    [Arguments]    ${velocity_rad_s}=1.0  ${amplitude_amps}=1.0
    [Documentation]    Enable synthetic 3-phase sinusoidal currents
    ...               Simulates motor running at given velocity
    
    # Enable motion (bit 0 of control register)
    Execute Command    sysbus.adcMock WriteDoubleWord 0x10 0x01
    Log    ADC Mock: Synthetic motion enabled @ ${velocity_rad_s} rad/s

Disable ADC Synthetic Motion
    [Documentation]    Disable synthetic motion, set currents to zero
    Execute Command    sysbus.adcMock WriteDoubleWord 0x10 0x00
    Log    ADC Mock: Synthetic motion disabled

Read ADC Phase Current
    [Arguments]    ${phase}
    [Documentation]    Read current ADC value for phase
    ${reg_offset}=    Set Variable If
    ...    '${phase}' == 'A'    0x00
    ...    '${phase}' == 'B'    0x04
    ...    '${phase}' == 'C'    0x08
    
    ${raw_value}=    Execute Command    sysbus.adcMock ReadDoubleWord ${reg_offset}
    [Return]    ${raw_value}

Inject ADC Overcurrent
    [Arguments]    ${phase}=A
    [Documentation]    Inject overcurrent condition on specified phase
    
    # Set current to 20A (way above typical limits)
    Set ADC Phase Current    ${phase}    20.0
    Log    Injected overcurrent on phase ${phase}

# ============================================================================
# Encoder Mock Control
# ============================================================================

Set Encoder Angle
    [Arguments]    ${angle_deg}
    [Documentation]    Set encoder absolute angle in degrees (0-360)
    
    # Convert to 15-bit raw value (0-32767)
    ${angle_raw}=    Evaluate    int((${angle_deg} % 360.0) / 360.0 * 32767)
    
    Execute Command    sysbus.encoderMock WriteDoubleWord 0x00 ${angle_raw}
    Log    Encoder Mock: Set angle to ${angle_deg}° (raw=${angle_raw})

Set Encoder Velocity
    [Arguments]    ${velocity_deg_s}
    [Documentation]    Set encoder angular velocity in degrees/second
    
    # Convert to millidegrees/sec for storage
    ${velocity_mdeg}=    Evaluate    int(${velocity_deg_s} * 1000)
    
    Execute Command    sysbus.encoderMock WriteDoubleWord 0x04 ${velocity_mdeg}
    Log    Encoder Mock: Set velocity to ${velocity_deg_s}°/s

Enable Encoder Motion
    [Arguments]    ${velocity_deg_s}=10.0
    [Documentation]    Enable continuous encoder rotation
    
    # Set velocity first
    Set Encoder Velocity    ${velocity_deg_s}
    
    # Enable motion (bit 0 of control register)
    Execute Command    sysbus.encoderMock WriteDoubleWord 0x08 0x01
    Log    Encoder Mock: Motion enabled @ ${velocity_deg_s}°/s

Disable Encoder Motion
    [Documentation]    Disable encoder motion
    Execute Command    sysbus.encoderMock WriteDoubleWord 0x08 0x00
    Log    Encoder Mock: Motion disabled

Read Encoder Angle
    [Documentation]    Read current encoder angle (raw 15-bit value)
    ${angle_raw}=    Execute Command    sysbus.encoderMock ReadDoubleWord 0x00
    
    # Convert to degrees
    ${angle_deg}=    Evaluate    (int('${angle_raw}') / 32767.0) * 360.0
    Log    Encoder angle: ${angle_deg}° (raw=${angle_raw})
    [Return]    ${angle_deg}

Inject Encoder Error
    [Arguments]    ${error_type}=1
    [Documentation]    Inject encoder error
    ...               error_type: 1=bad CRC, 2=timeout, 3=invalid data
    
    # Set error type
    Execute Command    sysbus.encoderMock WriteDoubleWord 0x0C ${error_type}
    
    # Enable error injection (bit 1 of control register)
    ${control}=    Execute Command    sysbus.encoderMock ReadDoubleWord 0x08
    ${control}=    Evaluate    int('${control}') | 2
    Execute Command    sysbus.encoderMock WriteDoubleWord 0x08 ${control}
    
    Log    Encoder Mock: Injected error type ${error_type}

Clear Encoder Error
    [Documentation]    Clear error injection
    Execute Command    sysbus.encoderMock WriteDoubleWord 0x0C 0
    
    ${control}=    Execute Command    sysbus.encoderMock ReadDoubleWord 0x08
    ${control}=    Evaluate    int('${control}') & ~2
    Execute Command    sysbus.encoderMock WriteDoubleWord 0x08 ${control}

# ============================================================================
# Utility Keywords
# ============================================================================

Setup Nominal Operating Conditions
    [Documentation]    Set all mocks to nominal operating conditions
    
    # ADC: Zero current, 48V DC bus
    Set ADC Phase Current    A    0.0
    Set ADC Phase Current    B    0.0
    Set ADC Phase Current    C    0.0
    Set ADC DC Voltage    48.0
    
    # Encoder: Zero angle, stopped
    Set Encoder Angle    0.0
    Disable Encoder Motion
    
    Log    Nominal operating conditions set

Setup Running Motor Conditions
    [Arguments]    ${velocity_deg_s}=30.0  ${current_amps}=2.0
    [Documentation]    Simulate motor running at specified speed
    
    # Enable synthetic 3-phase currents
    Enable ADC Synthetic Motion    velocity_rad_s=${velocity_deg_s * 0.0174533}    amplitude_amps=${current_amps}
    
    # Enable encoder motion
    Enable Encoder Motion    ${velocity_deg_s}
    
    # Nominal DC bus voltage
    Set ADC DC Voltage    48.0
    
    Log    Motor running: ${velocity_deg_s}°/s, ${current_amps}A

Wait For Encoder Angle
    [Arguments]    ${target_deg}  ${tolerance_deg}=5.0  ${timeout_sec}=5.0
    [Documentation]    Wait until encoder reaches target angle (within tolerance)
    
    FOR    ${i}    IN RANGE    50
        ${angle}=    Read Encoder Angle
        ${diff}=    Evaluate    abs(${angle} - ${target_deg})
        Return From Keyword If    ${diff} < ${tolerance_deg}
        Sleep    0.1s
    END
    
    Fail    Encoder did not reach target angle ${target_deg}° within ${timeout_sec}s


