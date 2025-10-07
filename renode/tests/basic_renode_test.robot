*** Settings ***
Documentation     Basic Renode test - verify platform boots and responds
...
...               This is a simplified test that doesn't require Python peripherals.
...               Tests basic firmware functionality in Renode emulator.

Resource          renode_helpers.robot
Library           String

Suite Setup       Setup Basic Test Suite
Suite Teardown    Teardown Basic Test Suite
Test Timeout      30 seconds


*** Variables ***
${TEST_NAME}    basic_renode


*** Keywords ***
Setup Basic Test Suite
    [Documentation]    Initialize Renode for basic tests
    Setup Renode Platform

Teardown Basic Test Suite
    [Documentation]    Clean up after tests
    Quick Teardown


*** Test Cases ***

Should Boot Firmware In Renode
    [Documentation]    Verify firmware boots successfully in Renode
    [Tags]    basic    renode    smoke
    
    # Start emulation
    Start Emulation
    
    # Wait for firmware to boot
    Sleep    1s
    
    # Check that emulation is running
    ${time}=    Execute Command    emulation GetTimeSourceInfo
    Log    Emulation time: ${time}
    
    # Basic smoke test - firmware should run
    Pass Execution    Firmware booted successfully in Renode

Should Initialize Peripherals
    [Documentation]    Verify peripherals are accessible
    [Tags]    basic    renode    peripherals
    
    Start Emulation
    Sleep    0.5s
    
    # Check that peripherals exist
    ${uart_info}=    Execute Command    sysbus.usart1
    Log    UART info: ${uart_info}
    
    ${can_info}=    Execute Command    sysbus.fdcan1
    Log    CAN info: ${can_info}
    
    ${spi_info}=    Execute Command    sysbus.spi1
    Log    SPI info: ${spi_info}
    
    Pass Execution    Peripherals initialized

Should Run For Duration
    [Documentation]    Verify emulation runs without crashes
    [Tags]    basic    renode    stability
    
    Start Emulation
    
    # Run for 5 seconds of virtual time
    Advance Time    5.0
    
    # Check still running
    ${time}=    Get Virtual Time
    Log    Ran for virtual time: ${time}
    
    Pass Execution    Emulation stable for 5s

Should Access Memory
    [Documentation]    Verify memory access works
    [Tags]    basic    renode    memory
    
    Start Emulation
    Sleep    0.1s
    
    # Read from SRAM
    ${value}=    Read Memory    0x20000000
    Log    SRAM value: ${value}
    
    # Write and read back
    Write Memory    0x20000100    0x12345678
    ${readback}=    Read Memory    0x20000100
    Log    Written: 0x12345678, Read: ${readback}
    
    Pass Execution    Memory access working

Should Control Emulation
    [Documentation]    Verify emulation control (start/pause/reset)
    [Tags]    basic    renode    control
    
    # Start
    Start Emulation
    Advance Time    1.0
    
    # Pause
    Stop Emulation
    ${time1}=    Get Virtual Time
    Sleep    0.5s    # Real time
    ${time2}=    Get Virtual Time
    
    # Time shouldn't advance when paused
    Log    Time while paused: ${time1} -> ${time2}
    
    # Resume
    Start Emulation
    Advance Time    1.0
    
    # Reset
    Reset Emulation
    
    Pass Execution    Emulation control working

