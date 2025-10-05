*** Settings ***
Documentation     CAN-FD communication tests with iRPC protocol
Suite Setup       Setup
Suite Teardown    Teardown
Test Setup        Reset Emulation
Resource          ${RENODEKEYWORDS}

*** Variables ***
${UART}                     sysbus.usart1
${FDCAN}                    sysbus.fdcan1
${SCRIPT}                   ${CURDIR}/../stm32g431_foc.resc
${ELF}                      ${CURDIR}/../../target/thumbv7em-none-eabihf/release/joint_firmware

*** Test Cases ***
Should Initialize FDCAN Peripheral
    [Documentation]         FDCAN1 should initialize successfully
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"
    
    # Check FDCAN is present
    ${peripherals}=         Execute Command    sysbus WhatPeripheralsAreEnabled
    Should Contain          ${peripherals}     fdcan1

Should Create CAN Hub
    [Documentation]         CAN hub for multi-device testing should work
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         emulation CreateCANHub "testBus"
    Execute Command         connector Connect ${FDCAN} testBus
    Execute Command         sysbus LoadELF $elf
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:00.1"

Should Handle CAN Frame Reception
    [Documentation]         Firmware should process received CAN frames
    Execute Command         $elf = @${ELF}
    Execute Command         mach create
    Execute Command         machine LoadPlatformDescription @${CURDIR}/../stm32g431cb.repl
    Execute Command         emulation CreateCANHub "testBus"
    Execute Command         connector Connect ${FDCAN} testBus
    Execute Command         sysbus LoadELF $elf
    Create Terminal Tester  ${UART}
    Start Emulation
    
    Execute Command         emulation RunFor "00:00:01"
    
    # Send test CAN frame (iRPC message)
    # Frame format: node_id=0x0010, lifecycle command
    Execute Command         ${FDCAN} SendFrame 0x10 0x01 0x00 0x00 0x00

    # Should process the frame
    Execute Command         emulation RunFor "00:00:00.01"

